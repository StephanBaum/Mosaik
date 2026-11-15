//! UI Automation reads: the focused element, the element at a point, element
//! comparison and the parent walk, through one client with connection and
//! transaction timeouts (ADR-0157 D1, D5).
#![expect(unsafe_code, reason = "Win32 and COM calls, ADR-0157")]

use core::ffi::c_void;
use core::marker::PhantomData;
use std::rc::Rc;

use windows::Win32::Foundation::{E_POINTER, POINT};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    SAFEARRAY,
};
use windows::Win32::System::Ole::{
    SafeArrayDestroy, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation2, IUIAutomationCacheRequest, IUIAutomationElement,
    IUIAutomationTreeWalker, IUIAutomationValuePattern, UIA_BoundingRectanglePropertyId,
    UIA_ControlTypePropertyId, UIA_FrameworkIdPropertyId, UIA_IsEnabledPropertyId,
    UIA_IsKeyboardFocusablePropertyId, UIA_IsOffscreenPropertyId, UIA_IsPasswordPropertyId,
    UIA_NativeWindowHandlePropertyId, UIA_ProcessIdPropertyId, UIA_ValuePatternId,
};
use windows::core::BSTR;

use crate::error::{Result, ShimError};

/// Serialises client start-up (see [`Uia::new`]).
static START: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The most parent steps any walk takes.
const MAX_PARENT_STEPS: usize = 64;
/// The longest runtime ID accepted (real ones have a handful of entries).
const MAX_RUNTIME_ID: usize = 64;

/// What the shim reads of an element, in one cached request.
#[derive(Clone, PartialEq, Eq, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per UIA fact the re-verification reads"
)]
pub struct ElementFacts {
    /// The UIA control type ID (`50004` is an edit).
    pub control_type: i32,
    /// Whether the element is a password field.
    pub is_password: bool,
    /// Whether the element is enabled.
    pub enabled: bool,
    /// Whether the element is off screen.
    pub offscreen: bool,
    /// Whether the element can take keyboard focus.
    pub keyboard_focusable: bool,
    /// The bounding rectangle: left, top, right, bottom, in screen pixels.
    pub rect: [i32; 4],
    /// The process that owns the element (for embedded web content this is
    /// not the window's owner).
    pub pid: u32,
    /// The native window handle value, `0` if the element has none.
    pub native_window: isize,
    /// The UIA runtime ID.
    pub runtime_id: Vec<i32>,
    /// The UI framework (`Win32`, `WPF`, `XAML`, `Chrome`, ...), for display
    /// and statistics only.
    pub framework_id: String,
    /// Whether the element supports the value pattern.
    pub has_value_pattern: bool,
    /// Whether the value pattern is read-only (false without the pattern).
    pub value_read_only: bool,
}

/// Balances a successful `CoInitializeEx` on the same thread. Declared last in
/// every struct that holds COM objects, so those are released first.
struct ComGuard {
    _not_send: PhantomData<*const ()>,
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: this guard exists only after a `CoInitializeEx` that
        // succeeded on this thread, and is `!Send`, so it drops on the same
        // thread; one uninitialize balances that one initialize.
        unsafe { CoUninitialize() };
    }
}

/// A UI Automation client. Not `Send`: it lives in the COM multithreaded
/// apartment of the thread that created it, which it initialises.
pub struct Uia {
    automation: IUIAutomation2,
    cache: IUIAutomationCacheRequest,
    walker: IUIAutomationTreeWalker,
    com: Rc<ComGuard>,
}

/// An element read through a [`Uia`], with its facts cached at read time.
/// Keeps its thread's COM initialisation alive until it is dropped.
pub struct UiaElement {
    element: IUIAutomationElement,
    facts: ElementFacts,
    _com: Rc<ComGuard>,
}

impl core::fmt::Debug for UiaElement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UiaElement")
            .field("pid", &self.facts.pid)
            .finish_non_exhaustive()
    }
}

impl UiaElement {
    /// The facts read when the element was fetched.
    #[must_use]
    pub fn facts(&self) -> &ElementFacts {
        &self.facts
    }
}

impl Uia {
    /// Starts a client with the given timeouts in milliseconds.
    ///
    /// # Errors
    /// An OS error; [`ShimError::Os`] if the thread is already in a
    /// single-threaded COM apartment.
    pub fn new(connection_timeout_ms: u32, transaction_timeout_ms: u32) -> Result<Self> {
        // UI Automation's first use in a process is not safe to run on several
        // threads at once: `CreateCacheRequest` then fails with E_FAIL.
        let _serialised = START
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // SAFETY: no reserved argument; joins the multithreaded apartment of
        // this thread. Both S_OK and S_FALSE must be balanced by an uninit.
        let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if init.is_err() {
            return Err(ShimError::from_hresult(init.0));
        }
        let com = Rc::new(ComGuard {
            _not_send: PhantomData,
        });
        // SAFETY: the class ID is the static CUIAutomation8; no aggregation.
        let automation: IUIAutomation2 =
            unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) }?;
        // SAFETY: `automation` is a live client; the call sets a timeout.
        unsafe { automation.SetConnectionTimeout(connection_timeout_ms) }?;
        // SAFETY: as above.
        unsafe { automation.SetTransactionTimeout(transaction_timeout_ms) }?;
        // SAFETY: as above; creates a cache request object.
        let cache = unsafe { automation.CreateCacheRequest() }?;
        for property in [
            UIA_ControlTypePropertyId,
            UIA_IsPasswordPropertyId,
            UIA_IsEnabledPropertyId,
            UIA_IsOffscreenPropertyId,
            UIA_IsKeyboardFocusablePropertyId,
            UIA_BoundingRectanglePropertyId,
            UIA_ProcessIdPropertyId,
            UIA_NativeWindowHandlePropertyId,
            UIA_FrameworkIdPropertyId,
        ] {
            // SAFETY: `cache` is a live cache request; the ID is a UIA constant.
            unsafe { cache.AddProperty(property) }?;
        }
        // SAFETY: as above.
        unsafe { cache.AddPattern(UIA_ValuePatternId) }?;
        // SAFETY: `automation` is a live client; returns the control-view walker.
        let walker = unsafe { automation.ControlViewWalker() }?;
        Ok(Self {
            automation,
            cache,
            walker,
            com,
        })
    }

    /// The element with keyboard focus, `None` if there is none.
    ///
    /// # Errors
    /// [`ShimError::Timeout`] if the provider does not answer, or an OS error.
    pub fn focused(&self) -> Result<Option<UiaElement>> {
        // SAFETY: `automation` and `cache` are live; the call reads focus.
        let found = unsafe { self.automation.GetFocusedElementBuildCache(&self.cache) };
        self.wrap(found)
    }

    /// The element at the screen point, `None` if there is none.
    ///
    /// # Errors
    /// [`ShimError::Timeout`] if the provider does not answer, or an OS error.
    pub fn element_from_point(&self, x: i32, y: i32) -> Result<Option<UiaElement>> {
        // SAFETY: `automation` and `cache` are live; the point is plain data.
        let found = unsafe {
            self.automation
                .ElementFromPointBuildCache(POINT { x, y }, &self.cache)
        };
        self.wrap(found)
    }

    /// Whether two elements are the same UI element (`CompareElements`).
    ///
    /// # Errors
    /// An OS error.
    pub fn same(&self, a: &UiaElement, b: &UiaElement) -> Result<bool> {
        // SAFETY: `automation` is live; both arguments are live elements.
        let same = unsafe { self.automation.CompareElements(&a.element, &b.element) }?;
        Ok(same.as_bool())
    }

    /// Whether `e` is `ancestor` or one of its descendants, looking at most
    /// [`MAX_PARENT_STEPS`] parents up.
    ///
    /// # Errors
    /// An OS error.
    pub fn is_within(&self, e: &UiaElement, ancestor: &UiaElement) -> Result<bool> {
        let mut current = e.element.clone();
        for _ in 0..=MAX_PARENT_STEPS {
            // SAFETY: `automation` is live; both arguments are live elements.
            let same = unsafe { self.automation.CompareElements(&current, &ancestor.element) }?;
            if same.as_bool() {
                return Ok(true);
            }
            match self.parent_of(&current)? {
                Some(parent) => current = parent,
                None => return Ok(false),
            }
        }
        Ok(false)
    }

    /// The native window handle of `e` or of its nearest ancestor that has
    /// one, looking at most [`MAX_PARENT_STEPS`] parents up.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] if none has a window; an OS error.
    pub fn nearest_window(&self, e: &UiaElement) -> Result<isize> {
        let mut current = e.element.clone();
        for _ in 0..=MAX_PARENT_STEPS {
            // SAFETY: `current` is a live element read with a cache request
            // that includes the native window handle.
            let hwnd = unsafe { current.CachedNativeWindowHandle() }?;
            if !hwnd.0.is_null() {
                return Ok(hwnd.0.addr().cast_signed());
            }
            match self.parent_of(&current)? {
                Some(parent) => current = parent,
                None => break,
            }
        }
        Err(ShimError::NotFound)
    }

    /// The parent of `element` in the control view, with the cache request
    /// applied; `None` at the root.
    fn parent_of(&self, element: &IUIAutomationElement) -> Result<Option<IUIAutomationElement>> {
        // SAFETY: `walker` and `cache` are live; `element` is a live element.
        let parent = unsafe { self.walker.GetParentElementBuildCache(element, &self.cache) };
        match parent {
            Ok(parent) => Ok(Some(parent)),
            Err(err) if absent(&err) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn wrap(
        &self,
        found: windows::core::Result<IUIAutomationElement>,
    ) -> Result<Option<UiaElement>> {
        match found {
            Ok(element) => {
                let facts = read_facts(&element)?;
                Ok(Some(UiaElement {
                    element,
                    facts,
                    _com: Rc::clone(&self.com),
                }))
            }
            Err(err) if absent(&err) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }
}

/// Whether the error means "no such element" rather than a failure.
fn absent(err: &windows::core::Error) -> bool {
    err.code() == E_POINTER || ShimError::from(err.clone()) == ShimError::NotFound
}

/// Reads the cached facts of an element, and its runtime ID.
fn read_facts(e: &IUIAutomationElement) -> Result<ElementFacts> {
    // SAFETY (all `Cached*` reads below): `e` is a live element fetched with
    // a cache request that includes the property, so each call only reads
    // the cache and returns a plain value.
    // SAFETY: see above.
    let control_type = unsafe { e.CachedControlType() }?.0;
    // SAFETY: see above.
    let is_password = unsafe { e.CachedIsPassword() }?.as_bool();
    // SAFETY: see above.
    let enabled = unsafe { e.CachedIsEnabled() }?.as_bool();
    // SAFETY: see above.
    let offscreen = unsafe { e.CachedIsOffscreen() }?.as_bool();
    // SAFETY: see above.
    let keyboard_focusable = unsafe { e.CachedIsKeyboardFocusable() }?.as_bool();
    // SAFETY: see above.
    let rect = unsafe { e.CachedBoundingRectangle() }?;
    // SAFETY: see above.
    let pid = unsafe { e.CachedProcessId() }?;
    // SAFETY: see above.
    let hwnd = unsafe { e.CachedNativeWindowHandle() }?;
    // SAFETY: see above.
    let framework: BSTR = unsafe { e.CachedFrameworkId() }?;
    let framework_id = String::from_utf16(&framework).map_err(|_| ShimError::Os(0))?;
    // SAFETY: `e` is live; the pattern was requested in the cache request. An
    // element without the pattern gives an error, which means "no pattern".
    let pattern = unsafe { e.GetCachedPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) };
    let (has_value_pattern, value_read_only) = match pattern {
        Ok(pattern) => {
            // SAFETY: `pattern` is a live pattern object from the cache.
            let read_only = unsafe { pattern.CachedIsReadOnly() }.map(windows::core::BOOL::as_bool);
            (true, read_only.unwrap_or(true))
        }
        Err(_) => (false, false),
    };
    Ok(ElementFacts {
        control_type,
        is_password,
        enabled,
        offscreen,
        keyboard_focusable,
        rect: [rect.left, rect.top, rect.right, rect.bottom],
        pid: u32::try_from(pid).map_err(|_| ShimError::Os(0))?,
        native_window: hwnd.0.addr().cast_signed(),
        runtime_id: runtime_id(e)?,
        framework_id,
        has_value_pattern,
        value_read_only,
    })
}

/// Owns a `SAFEARRAY` the provider returned, destroyed on drop.
struct SafeArray(*mut SAFEARRAY);

impl Drop for SafeArray {
    fn drop(&mut self) {
        // SAFETY: the pointer came from a successful `GetRuntimeId`, which
        // transfers ownership of the array to the caller; drop runs once.
        let destroyed = unsafe { SafeArrayDestroy(self.0) };
        drop(destroyed);
    }
}

fn runtime_id(e: &IUIAutomationElement) -> Result<Vec<i32>> {
    // SAFETY: `e` is a live element; the call returns a new array.
    let array = SafeArray(unsafe { e.GetRuntimeId() }?);
    if array.0.is_null() {
        return Err(ShimError::NotFound);
    }
    // SAFETY: `array.0` is a valid one-dimensional SAFEARRAY; dimension 1.
    let lower = unsafe { SafeArrayGetLBound(array.0, 1) }?;
    // SAFETY: as above.
    let upper = unsafe { SafeArrayGetUBound(array.0, 1) }?;
    let count =
        usize::try_from(i64::from(upper) - i64::from(lower) + 1).map_err(|_| ShimError::Os(0))?;
    if count > MAX_RUNTIME_ID {
        return Err(ShimError::Os(0));
    }
    let mut ids = Vec::with_capacity(count);
    for index in lower..=upper {
        let mut value = 0i32;
        // SAFETY: `array.0` is a valid SAFEARRAY of `i32` (runtime IDs are
        // integers); `index` is within its bounds; `value` is a writable
        // `i32`, the element size.
        unsafe {
            SafeArrayGetElement(array.0, &raw const index, (&raw mut value).cast::<c_void>())
        }?;
        ids.push(value);
    }
    Ok(ids)
}

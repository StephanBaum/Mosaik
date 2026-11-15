//! UI Automation reads: the focused element, the element at a point, element
//! comparison and the parent walk, through one client with connection and
//! transaction timeouts (ADR-0157 D1, D5).
#![expect(unsafe_code, reason = "Win32 and COM calls, ADR-0157")]

use core::ffi::c_void;
use core::marker::PhantomData;
use std::rc::Rc;
use std::sync::mpsc::SyncSender;
use std::time::Instant;

use windows::Win32::Foundation::{E_POINTER, HWND, POINT};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    SAFEARRAY,
};
use windows::Win32::System::Ole::{
    SafeArrayDestroy, SafeArrayGetDim, SafeArrayGetElement, SafeArrayGetElemsize,
    SafeArrayGetLBound, SafeArrayGetUBound, SafeArrayGetVartype,
};
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation2, IUIAutomationCacheRequest, IUIAutomationCondition,
    IUIAutomationElement, IUIAutomationFocusChangedEventHandler, IUIAutomationTreeWalker,
    IUIAutomationValuePattern, TreeScope_Descendants, UIA_BoundingRectanglePropertyId,
    UIA_ControlTypePropertyId, UIA_EditControlTypeId, UIA_FrameworkIdPropertyId,
    UIA_IsEnabledPropertyId, UIA_IsKeyboardFocusablePropertyId, UIA_IsOffscreenPropertyId,
    UIA_IsPasswordPropertyId, UIA_NativeWindowHandlePropertyId, UIA_ProcessIdPropertyId,
    UIA_ValueIsReadOnlyPropertyId, UIA_ValuePatternId,
};
use windows::core::BSTR;

use crate::error::{Result, ShimError, check_deadline};
use crate::sys_window::window_facts;

/// Serialises client start-up (see [`Uia::new`]).
static START: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The most parent steps any walk takes.
const MAX_PARENT_STEPS: usize = 64;
/// The most elements a bounded window walk looks at; more is [`ShimError::Timeout`]
/// (an answer of "not decided", never a partial list passed off as complete).
const MAX_WALK_NODES: usize = 4000;
/// The longest runtime ID accepted (real ones have a handful of entries).
const MAX_RUNTIME_ID: usize = 64;
/// The longest provider string (the framework ID) copied, in UTF-16 units; a
/// longer one is an error, not a truncation.
const MAX_PROVIDER_STRING_UNITS: usize = 256;
/// The longest value read back from a field, in UTF-16 units; a longer one is an
/// error, not a truncation.
const MAX_VALUE_UNITS: usize = 8192;
/// The properties every element read caches (one request, no later calls).
pub(crate) const CACHED_PROPERTIES: [windows::Win32::UI::Accessibility::UIA_PROPERTY_ID; 10] = [
    UIA_ControlTypePropertyId,
    UIA_IsPasswordPropertyId,
    UIA_IsEnabledPropertyId,
    UIA_IsOffscreenPropertyId,
    UIA_IsKeyboardFocusablePropertyId,
    UIA_BoundingRectanglePropertyId,
    UIA_ProcessIdPropertyId,
    UIA_NativeWindowHandlePropertyId,
    UIA_FrameworkIdPropertyId,
    // `IUIAutomationValuePattern::CachedIsReadOnly` reads this property; the
    // pattern request alone does not cache it.
    UIA_ValueIsReadOnlyPropertyId,
];

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
        for property in CACHED_PROPERTIES {
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
    /// [`ShimError::Timeout`] if the provider does not answer or `deadline`
    /// has passed, or an OS error.
    pub fn focused(&self, deadline: Instant) -> Result<Option<UiaElement>> {
        check_deadline(deadline)?;
        // SAFETY: `automation` and `cache` are live; the call reads focus.
        let found = unsafe { self.automation.GetFocusedElementBuildCache(&self.cache) };
        self.wrap(found)
    }

    /// The element at the screen point, `None` if there is none.
    ///
    /// # Errors
    /// [`ShimError::Timeout`] if the provider does not answer or `deadline`
    /// has passed, or an OS error.
    pub fn element_from_point(
        &self,
        x: i32,
        y: i32,
        deadline: Instant,
    ) -> Result<Option<UiaElement>> {
        check_deadline(deadline)?;
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
    /// [`ShimError::Timeout`] past `deadline`, or an OS error.
    pub fn same(&self, a: &UiaElement, b: &UiaElement, deadline: Instant) -> Result<bool> {
        check_deadline(deadline)?;
        // SAFETY: `automation` is live; both arguments are live elements.
        let same = unsafe { self.automation.CompareElements(&a.element, &b.element) }?;
        Ok(same.as_bool())
    }

    /// Whether `e` is `ancestor` or one of its descendants, looking at most
    /// [`MAX_PARENT_STEPS`] parents up.
    ///
    /// # Errors
    /// [`ShimError::Timeout`] past `deadline` (checked before every step), or
    /// an OS error.
    pub fn is_within(
        &self,
        e: &UiaElement,
        ancestor: &UiaElement,
        deadline: Instant,
    ) -> Result<bool> {
        let mut current = e.element.clone();
        for _ in 0..=MAX_PARENT_STEPS {
            check_deadline(deadline)?;
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
    /// [`ShimError::NotFound`] if none has a window, [`ShimError::Timeout`]
    /// past `deadline` (checked before every step); an OS error.
    pub fn nearest_window(&self, e: &UiaElement, deadline: Instant) -> Result<isize> {
        let mut current = e.element.clone();
        for _ in 0..=MAX_PARENT_STEPS {
            check_deadline(deadline)?;
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

    /// Subscribes to focus changes. The handler sends the process ID of each
    /// newly focused element (`0` if it cannot be read) with `try_send`, so a
    /// full channel drops the event and a UI Automation thread never blocks.
    ///
    /// # Errors
    /// An OS error.
    pub fn subscribe_focus(&self, sink: FocusSink) -> Result<FocusSubscription> {
        let handler: IUIAutomationFocusChangedEventHandler = FocusHandler { sink }.into();
        // SAFETY: `automation` is live; no cache request is passed (the
        // handler reads the sender live); `handler` is a live COM object that
        // the call adds a reference to.
        unsafe {
            self.automation
                .AddFocusChangedEventHandler(None::<&IUIAutomationCacheRequest>, &handler)
        }?;
        Ok(FocusSubscription {
            automation: self.automation.clone(),
            handler,
            _com: Rc::clone(&self.com),
        })
    }

    /// The Edit controls under the ancestor `levels_up` steps above `e` (the
    /// climb stops at a top-level window or the root), in tree order, at most
    /// `max`.
    ///
    /// # Errors
    /// [`ShimError::Timeout`] if a provider does not answer, or an OS error.
    pub fn edits_near(&self, e: &UiaElement, levels_up: u8, max: usize) -> Result<Vec<UiaElement>> {
        let mut current = e.element.clone();
        for _ in 0..levels_up {
            // SAFETY: `current` is a live element read with a cache request
            // that includes the native window handle.
            let hwnd = unsafe { current.CachedNativeWindowHandle() }?;
            let addr = hwnd.0.addr().cast_signed();
            if addr != 0 && window_facts(addr).is_ok_and(|w| w.root == addr) {
                break;
            }
            match self.parent_of(&current)? {
                Some(parent) => current = parent,
                None => break,
            }
        }
        self.edits_under(&current, max)
    }

    /// The Edit controls of the window `root`, in tree order, at most `max`.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] for a window that does not exist,
    /// [`ShimError::Timeout`], or an OS error.
    pub fn edits_in_window(&self, root: isize, max: usize) -> Result<Vec<UiaElement>> {
        window_facts(root)?;
        let hwnd = HWND(core::ptr::without_provenance_mut::<c_void>(
            root.cast_unsigned(),
        ));
        // SAFETY: `automation` and `cache` are live; `hwnd` was just checked
        // to be a window, and UI Automation validates it again.
        let element = unsafe {
            self.automation
                .ElementFromHandleBuildCache(hwnd, &self.cache)
        }?;
        self.edits_under(&element, max)
    }

    /// The Edit controls of the window `root` in tree order, found by a walk
    /// that checks `deadline` before every element and stops as soon as
    /// `stop_after` edits satisfy `counts` (or `max` edits are held). A
    /// window larger than [`MAX_WALK_NODES`] elements is a timeout, never a
    /// silently truncated list.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] for a window that does not exist,
    /// [`ShimError::Timeout`] past `deadline` or for a too large tree, or an
    /// OS error.
    pub fn edits_in_window_until(
        &self,
        root: isize,
        max: usize,
        deadline: Instant,
        stop_after: usize,
        counts: &dyn Fn(&ElementFacts) -> bool,
    ) -> Result<Vec<UiaElement>> {
        window_facts(root)?;
        let hwnd = HWND(core::ptr::without_provenance_mut::<c_void>(
            root.cast_unsigned(),
        ));
        // SAFETY: `automation` and `cache` are live; `hwnd` was just checked
        // to be a window, and UI Automation validates it again.
        let top = unsafe {
            self.automation
                .ElementFromHandleBuildCache(hwnd, &self.cache)
        }?;
        let mut out = Vec::new();
        let mut counted = 0usize;
        let mut visited = 0usize;
        let mut pending: Vec<IUIAutomationElement> = Vec::new();
        if let Some(first) = self.first_child_of(&top)? {
            pending.push(first);
        }
        while let Some(node) = pending.pop() {
            check_deadline(deadline)?;
            visited += 1;
            if visited > MAX_WALK_NODES {
                return Err(ShimError::Timeout);
            }
            // The sibling goes below the child, so the walk is in tree order.
            if let Some(next) = self.next_sibling_of(&node)? {
                pending.push(next);
            }
            // SAFETY: `node` is a live element fetched with a cache request
            // that includes the control type; the call reads the cache.
            let control_type = unsafe { node.CachedControlType() }?.0;
            if control_type == UIA_EditControlTypeId.0 {
                let facts = read_facts(&node)?;
                if counts(&facts) {
                    counted += 1;
                }
                out.push(UiaElement {
                    element: node.clone(),
                    facts,
                    _com: Rc::clone(&self.com),
                });
                if counted >= stop_after || out.len() >= max {
                    break;
                }
            }
            if let Some(child) = self.first_child_of(&node)? {
                pending.push(child);
            }
        }
        Ok(out)
    }

    fn first_child_of(
        &self,
        element: &IUIAutomationElement,
    ) -> Result<Option<IUIAutomationElement>> {
        // SAFETY: `walker` and `cache` are live; `element` is a live element.
        let child = unsafe {
            self.walker
                .GetFirstChildElementBuildCache(element, &self.cache)
        };
        match child {
            Ok(child) => Ok(Some(child)),
            Err(err) if absent(&err) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn next_sibling_of(
        &self,
        element: &IUIAutomationElement,
    ) -> Result<Option<IUIAutomationElement>> {
        // SAFETY: `walker` and `cache` are live; `element` is a live element.
        let next = unsafe {
            self.walker
                .GetNextSiblingElementBuildCache(element, &self.cache)
        };
        match next {
            Ok(next) => Ok(Some(next)),
            Err(err) if absent(&err) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// `e` read again, with fresh cached facts.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] if the element is gone, [`ShimError::Timeout`],
    /// or an OS error.
    pub fn refresh(&self, e: &UiaElement) -> Result<UiaElement> {
        // SAFETY: `e.element` and `cache` are live; the call reads the element.
        let updated = unsafe { e.element.BuildUpdatedCache(&self.cache) }?;
        let facts = read_facts(&updated)?;
        Ok(UiaElement {
            element: updated,
            facts,
            _com: Rc::clone(&self.com),
        })
    }

    /// Gives `e` keyboard focus.
    ///
    /// # Errors
    /// [`ShimError::NotFound`] if the element is gone, [`ShimError::Timeout`],
    /// or an OS error.
    pub fn set_focus(&self, e: &UiaElement) -> Result<()> {
        // SAFETY: `e.element` is a live element; the call asks it to take focus.
        unsafe { e.element.SetFocus() }?;
        Ok(())
    }

    /// Sets the value of a text element through the value pattern.
    ///
    /// # Errors
    /// [`ShimError::AccessDenied`] for a password element (never written
    /// this way), [`ShimError::NotFound`] without a value pattern,
    /// [`ShimError::Timeout`], or an OS error.
    pub fn set_value(&self, e: &UiaElement, value: &str) -> Result<()> {
        refuse_password(e)?;
        let pattern = value_pattern(e)?;
        let text = BSTR::from(value);
        // SAFETY: `pattern` is a live pattern object; `text` is a valid BSTR.
        unsafe { pattern.SetValue(&text) }?;
        Ok(())
    }

    /// Reads the value of a text element through the value pattern.
    ///
    /// # Errors
    /// As [`Uia::set_value`]; also an OS error for a value longer than the
    /// limit or not valid UTF-16.
    pub fn value(&self, e: &UiaElement) -> Result<String> {
        refuse_password(e)?;
        let pattern = value_pattern(e)?;
        // SAFETY: `pattern` is a live pattern object; the call returns a new BSTR.
        let text: BSTR = unsafe { pattern.CurrentValue() }?;
        if text.len() > MAX_VALUE_UNITS {
            return Err(ShimError::Os(0));
        }
        String::from_utf16(&text).map_err(|_| ShimError::Os(0))
    }

    /// The Edit controls below `root`, in tree order, at most `max`.
    fn edits_under(&self, root: &IUIAutomationElement, max: usize) -> Result<Vec<UiaElement>> {
        let value = VARIANT::from(UIA_EditControlTypeId.0);
        // SAFETY: `automation` is live; `value` is a valid VARIANT that the call copies.
        let condition: IUIAutomationCondition = unsafe {
            self.automation
                .CreatePropertyCondition(UIA_ControlTypePropertyId, &value)
        }?;
        // SAFETY: `root`, `condition` and `cache` are live; the call searches
        // the subtree and returns a new array.
        let found =
            unsafe { root.FindAllBuildCache(TreeScope_Descendants, &condition, &self.cache) }?;
        // SAFETY: `found` is a live element array.
        let length = unsafe { found.Length() }?;
        let count = usize::try_from(length).unwrap_or(0).min(max);
        let mut out = Vec::with_capacity(count);
        for index in 0..count {
            let index = i32::try_from(index).map_err(|_| ShimError::Os(0))?;
            // SAFETY: `found` is live and `index` is below its length.
            let element = unsafe { found.GetElement(index) }?;
            let facts = read_facts(&element)?;
            out.push(UiaElement {
                element,
                facts,
                _com: Rc::clone(&self.com),
            });
        }
        Ok(out)
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

/// Where focus-change events send the new element process ID (`0` when it
/// cannot be read). Use a bounded channel: the handler never blocks.
pub type FocusSink = SyncSender<u32>;

/// The COM object UI Automation calls on a focus change.
mod focus_handler {
    #![expect(
        clippy::ref_as_ptr,
        clippy::inline_always,
        reason = "generated by windows::core::implement"
    )]

    use std::sync::mpsc::SyncSender;

    use windows::Win32::UI::Accessibility::{
        IUIAutomationElement, IUIAutomationFocusChangedEventHandler,
        IUIAutomationFocusChangedEventHandler_Impl,
    };
    use windows::core::Ref;

    #[windows::core::implement(IUIAutomationFocusChangedEventHandler)]
    pub(super) struct FocusHandler {
        pub(super) sink: SyncSender<u32>,
    }

    impl IUIAutomationFocusChangedEventHandler_Impl for FocusHandler_Impl {
        fn HandleFocusChangedEvent(
            &self,
            sender: Ref<'_, IUIAutomationElement>,
        ) -> windows::core::Result<()> {
            let pid = sender
                .as_ref()
                // SAFETY: `element` is the live sender UI Automation passed in; the
                // call reads its process ID (bounded by the client timeouts).
                .and_then(|element| unsafe { element.CurrentProcessId() }.ok())
                .and_then(|pid| u32::try_from(pid).ok())
                .unwrap_or(0);
            // A full channel or a closed receiver drops the event.
            self.sink.try_send(pid).ok();
            Ok(())
        }
    }
}

use focus_handler::FocusHandler;

/// A live focus-change subscription. Not `Send`: it belongs to the thread that
/// created the client. Dropping it removes the handler.
pub struct FocusSubscription {
    automation: IUIAutomation2,
    handler: IUIAutomationFocusChangedEventHandler,
    _com: Rc<ComGuard>,
}

impl Drop for FocusSubscription {
    fn drop(&mut self) {
        // SAFETY: `automation` is live and `handler` is the one added when the
        // subscription was made; removing a handler twice or after the client
        // is gone only returns an error, which cannot be handled here.
        let removed = unsafe {
            self.automation
                .RemoveFocusChangedEventHandler(&self.handler)
        };
        drop(removed);
    }
}

/// Refuses password elements, read live (the cached fact could be stale).
fn refuse_password(e: &UiaElement) -> Result<()> {
    // SAFETY: `e.element` is a live element; the call reads a property.
    let live = unsafe { e.element.CurrentIsPassword() }?.as_bool();
    if e.facts.is_password || live {
        return Err(ShimError::AccessDenied);
    }
    Ok(())
}

fn value_pattern(e: &UiaElement) -> Result<IUIAutomationValuePattern> {
    // SAFETY: `e.element` is a live element; an element without the pattern
    // gives an error.
    unsafe {
        e.element
            .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
    }
    .map_err(|err| match ShimError::from(err) {
        ShimError::Timeout => ShimError::Timeout,
        _ => ShimError::NotFound,
    })
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
    if framework.len() > MAX_PROVIDER_STRING_UNITS {
        return Err(ShimError::Os(0));
    }
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
    // SAFETY: `array.0` is a valid, non-null SAFEARRAY; the call reads its
    // descriptor.
    let dims = unsafe { SafeArrayGetDim(array.0) };
    // SAFETY: as above.
    let vartype = unsafe { SafeArrayGetVartype(array.0) }?;
    // SAFETY: as above.
    let element_size = unsafe { SafeArrayGetElemsize(array.0) };
    // A provider could return any array: only a one-dimensional array of
    // 32-bit integers is read into `i32`.
    if dims != 1 || vartype != VT_I4 || element_size != 4 {
        return Err(ShimError::Os(0));
    }
    // SAFETY: `array.0` is a one-dimensional SAFEARRAY (checked above); dimension 1.
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
        // SAFETY: `array.0` is a valid SAFEARRAY of `i32` (checked above: one
        // dimension, `VT_I4`, element size 4); `index` is within its bounds; `value` is a writable
        // `i32`, the element size.
        unsafe {
            SafeArrayGetElement(array.0, &raw const index, (&raw mut value).cast::<c_void>())
        }?;
        ids.push(value);
    }
    Ok(ids)
}

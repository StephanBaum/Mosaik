import Observation
import SwiftUI
import UIKit

/// Whether the screen is being recorded, mirrored or shared (spec 3.13). While it is, revealed values are
/// hidden and reveal is disabled, with the note "Hidden while your screen is recorded". A screenshot cannot
/// be blocked on iOS (R129); it only hides what is revealed.
@MainActor
@Observable
public final class CaptureObserver {
    public private(set) var isCaptured: Bool
    /// Counts screenshots. A screen that reveals a value hides it when this changes.
    public private(set) var screenshotCount = 0
    @ObservationIgnored private var screenshotObserver: NSObjectProtocol?

    public init(isCaptured: Bool = false) {
        self.isCaptured = isCaptured
    }

    /// Reveal is allowed only while nothing captures the screen.
    public var revealAllowed: Bool { !isCaptured }

    /// The note to show instead of a value while captured.
    public var hiddenNote: String? { isCaptured ? L10n.captureHidden() : nil }

    public func update(isCaptured: Bool) {
        self.isCaptured = isCaptured
    }

    public func screenshotTaken() {
        screenshotCount += 1
    }

    /// Starts listening for screenshots. The capture state comes from `CaptureProbe`.
    public func start() {
        guard screenshotObserver == nil else { return }
        screenshotObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.userDidTakeScreenshotNotification, object: nil, queue: .main
        ) { [weak self] _ in
            MainActor.assumeIsolated {
                self?.screenshotTaken()
            }
        }
    }

    public func stop() {
        if let screenshotObserver {
            NotificationCenter.default.removeObserver(screenshotObserver)
        }
        screenshotObserver = nil
    }
}

/// A zero-size view that reports `sceneCaptureState` (iOS 17) of its scene to a `CaptureObserver`.
public struct CaptureProbe: UIViewRepresentable {
    private let observer: CaptureObserver

    public init(observer: CaptureObserver) {
        self.observer = observer
    }

    public func makeUIView(context: Context) -> ProbeView {
        let view = ProbeView()
        view.observer = observer
        view.isUserInteractionEnabled = false
        view.isAccessibilityElement = false
        return view
    }

    public func updateUIView(_ uiView: ProbeView, context: Context) {
        uiView.observer = observer
        uiView.report()
    }

    public final class ProbeView: UIView {
        weak var observer: CaptureObserver?

        override init(frame: CGRect) {
            super.init(frame: frame)
            registerForTraitChanges([UITraitSceneCaptureState.self], action: #selector(captureStateChanged))
        }

        @available(*, unavailable)
        required init?(coder: NSCoder) {
            fatalError("init(coder:) is not used")
        }

        @objc private func captureStateChanged() {
            report()
        }

        override public func didMoveToWindow() {
            super.didMoveToWindow()
            report()
        }

        func report() {
            observer?.update(isCaptured: traitCollection.sceneCaptureState == .active)
        }
    }
}

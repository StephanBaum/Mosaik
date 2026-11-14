import AVFoundation
import MosaicKit
import Observation
import SwiftUI
import UIKit

/// The QR scanner state (spec 3.14): `AVCaptureMetadataOutput` with `.qr` gives the payload string. The
/// camera never hands over a frame, so no frame is stored. A payload that is not for the core is dropped
/// unread and the screen says "Not a Mosaic setup code" at once. The permission is asked when the screen is
/// used, after a sentence that says why.
@MainActor
@Observable
final class ScanModel {
    let target: ScanTarget
    private(set) var access: CameraAccess
    private(set) var feedback: ScanFeedback = .looking

    @ObservationIgnored private let holder: CoreHolder?
    @ObservationIgnored private let onSetupCode: (SetupCode) -> Void
    @ObservationIgnored private let onOtp: (OtpPreview) -> Void
    @ObservationIgnored private var finished = false
    @ObservationIgnored private var inFlight = false
    @ObservationIgnored private(set) var controller: QRCaptureController?

    init(
        target: ScanTarget, holder: CoreHolder?, onSetupCode: @escaping (SetupCode) -> Void = { _ in },
        onOtp: @escaping (OtpPreview) -> Void = { _ in }
    ) {
        self.target = target
        self.holder = holder
        self.onSetupCode = onSetupCode
        self.onOtp = onOtp
        self.access = CameraAccess.current()
        self.controller = QRCaptureController { [weak self] payload in
            Task { @MainActor in
                self?.handle(payload: payload)
            }
        }
    }

    /// Asks for the camera once the user agreed.
    func requestAccess() async {
        let allowed = await AVCaptureDevice.requestAccess(for: .video)
        access = allowed ? .granted : .denied
        if allowed {
            start()
        }
    }

    /// The screen appeared or the app came back: read the permission again (the user may have changed it in
    /// the system settings) and start the camera when allowed.
    func appeared(active: Bool) {
        access = CameraAccess.current()
        if access == .granted && active {
            start()
        } else {
            controller?.stop()
        }
    }

    func stop() {
        controller?.stop()
    }

    private func start() {
        controller?.start { [weak self] in
            Task { @MainActor in
                self?.cameraFailed()
            }
        }
    }

    private func cameraFailed() {
        access = .unavailable
    }

    /// One QR payload from the camera. The core runs it off the main thread.
    func handle(payload: String) {
        guard !finished, !inFlight else { return }
        guard ScanPayload.verdict(for: payload, target: target) == .candidate else {
            feedback = target == .setupCode ? .notSetupCode : .error("otpQrNotOtp")
            return
        }
        guard let holder else { return }
        inFlight = true
        let target = self.target
        Task { [self] in
            defer { inFlight = false }
            do {
                switch target {
                case .setupCode:
                    // The bytes are made inside the call and wiped there: the setup code holds the Secret Key.
                    // (The `String` the camera API gave cannot be wiped, THREAT_MODEL R126.)
                    let code = try await holder.call { core in
                        var text = Data(payload.utf8)
                        defer { text.wipe() }
                        return try core.parseSetupCode(text: text)
                    }
                    finished = true
                    feedback = .looking
                    controller?.stop()
                    onSetupCode(code)
                case .otp:
                    let preview = try await holder.call { core in
                        var text = Data(payload.utf8)
                        defer { text.wipe() }
                        return try core.otpFromText(text: text)
                    }
                    finished = true
                    feedback = .looking
                    controller?.stop()
                    onOtp(preview)
                }
            } catch {
                feedback = .error(ErrorText.code(of: error))
            }
        }
    }
}

/// The camera, the permission states and what the scan says. Used on the onboarding screen and inside the
/// login editor.
struct ScanBody: View {
    let model: ScanModel
    let sceneActive: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: MosaicTokens.Space.s4) {
            switch model.access {
            case .unavailable:
                BodyText(text: L10n.scanUnavailable())
            case .ask:
                BodyText(text: L10n.scanRationale())
                Button(L10n.scanAllow()) { Task { await model.requestAccess() } }
                    .buttonStyle(PrimaryButtonStyle())
                    .accessibilityIdentifier("scan-allow")
            case .denied:
                BodyText(text: L10n.scanDenied())
                Button(L10n.scanOpenSettings()) { openAppSettings() }
                    .buttonStyle(PrimaryButtonStyle())
                    .accessibilityIdentifier("scan-open-settings")
            case .granted:
                if let controller = model.controller {
                    QRCameraView(controller: controller)
                        .aspectRatio(3.0 / 4.0, contentMode: .fit)
                        .frame(maxWidth: .infinity)
                        .clipShape(RoundedRectangle(cornerRadius: MosaicTokens.Radius.card))
                        .accessibilityIdentifier("scan-camera")
                }
                feedbackLine
            }
        }
        .onAppear { model.appeared(active: sceneActive) }
        .onChange(of: sceneActive) { _, active in model.appeared(active: active) }
        .onDisappear { model.stop() }
    }

    @ViewBuilder
    private var feedbackLine: some View {
        switch model.feedback {
        case .looking:
            HintText(text: model.target == .setupCode ? L10n.setupScanHint() : L10n.scanOtpHint())
        case .notSetupCode:
            ErrorLine(text: L10n.scanNotSetupCode())
            HintText(text: L10n.scanNotSetupCodeHelp())
        case let .error(code):
            ErrorLine(text: ErrorText.message(forCode: code))
        }
    }

    private func openAppSettings() {
        if let url = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(url)
        }
    }
}

/// The full onboarding screen of the setup-code scanner.
struct SetupScanScreen: View {
    let model: ScanModel
    let sceneActive: Bool
    let onBack: @MainActor () -> Void

    var body: some View {
        MosaicScreen {
            ScreenTitle(text: L10n.setupScanTitle())
            ScanBody(model: model, sceneActive: sceneActive)
            Button(L10n.commonBack(), action: onBack)
                .buttonStyle(SecondaryButtonStyle())
                .accessibilityIdentifier("scan-back")
        }
    }
}

/// The preview of the camera. The layer shows the session; nothing reads a frame.
struct QRCameraView: UIViewRepresentable {
    let controller: QRCaptureController

    func makeUIView(context: Context) -> CapturePreviewView {
        let view = CapturePreviewView()
        view.previewLayer.session = controller.previewSession
        view.previewLayer.videoGravity = .resizeAspectFill
        view.isAccessibilityElement = true
        view.accessibilityLabel = L10n.scanPreview()
        return view
    }

    func updateUIView(_ uiView: CapturePreviewView, context: Context) {}
}

final class CapturePreviewView: UIView {
    override class var layerClass: AnyClass { AVCaptureVideoPreviewLayer.self }

    var previewLayer: AVCaptureVideoPreviewLayer {
        unsafeDowncast(layer, to: AVCaptureVideoPreviewLayer.self)
    }
}

/// Owns the capture session. It starts and stops on its own queue; the metadata delegate answers on the main
/// queue, with the string of a QR code only.
final class QRCaptureController: NSObject, AVCaptureMetadataOutputObjectsDelegate, @unchecked Sendable {
    private let session = AVCaptureSession()
    private let queue = DispatchQueue(label: "io.github.stephanbaum.mosaic.scan", qos: .userInitiated)
    private let onPayload: @Sendable (String) -> Void
    /// Touched only on `queue`.
    private var configured = false

    init(onPayload: @escaping @Sendable (String) -> Void) {
        self.onPayload = onPayload
        super.init()
    }

    var previewSession: AVCaptureSession { session }

    func start(onFailure: @escaping @Sendable () -> Void) {
        queue.async { [self] in
            if !configured {
                guard configure() else {
                    onFailure()
                    return
                }
                configured = true
            }
            if !session.isRunning {
                session.startRunning()
            }
        }
    }

    func stop() {
        queue.async { [self] in
            if session.isRunning {
                session.stopRunning()
            }
        }
    }

    private func configure() -> Bool {
        guard
            let device = AVCaptureDevice.default(for: .video),
            let input = try? AVCaptureDeviceInput(device: device)
        else {
            return false
        }
        session.beginConfiguration()
        defer { session.commitConfiguration() }
        guard session.canAddInput(input) else { return false }
        session.addInput(input)
        let output = AVCaptureMetadataOutput()
        guard session.canAddOutput(output) else { return false }
        session.addOutput(output)
        output.setMetadataObjectsDelegate(self, queue: .main)
        output.metadataObjectTypes = [.qr]
        return true
    }

    func metadataOutput(
        _ output: AVCaptureMetadataOutput, didOutput metadataObjects: [AVMetadataObject],
        from connection: AVCaptureConnection
    ) {
        for object in metadataObjects {
            if let code = object as? AVMetadataMachineReadableCodeObject, code.type == .qr,
                let value = code.stringValue
            {
                onPayload(value)
                return
            }
        }
    }
}

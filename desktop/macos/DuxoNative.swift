import AppKit
import WebKit

final class Bridge: NSObject, WKScriptMessageHandler {
    weak var controller: DuxoController?

    init(controller: DuxoController) {
        self.controller = controller
    }

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard let body = message.body as? [String: Any],
              let type = body["type"] as? String else { return }

        if type == "request",
           let id = body["id"] as? String,
           let method = body["method"] as? String {
            let args = body["args"]
            controller?.handleRequest(id: id, method: method, args: args)
        }
    }
}

final class DuxoController: NSObject, NSApplicationDelegate, WKNavigationDelegate {
    private var window: NSWindow!
    private var webView: WKWebView!
    private var bridge: Bridge!
    private let version = "0.1.0"

    func applicationDidFinishLaunching(_ notification: Notification) {
        let configuration = WKWebViewConfiguration()
        let controller = WKUserContentController()
        bridge = Bridge(controller: self)
        controller.add(bridge, name: "duxo")
        controller.addUserScript(WKUserScript(
            source: Self.bridgeJavaScript,
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        ))
        configuration.userContentController = controller

        webView = WKWebView(frame: .zero, configuration: configuration)
        webView.navigationDelegate = self
        webView.setValue(false, forKey: "drawsBackground")

        window = NSWindow(
            contentRect: NSRect(x: 0, y: 0, width: 1180, height: 760),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "Duxo"
        window.minSize = NSSize(width: 900, height: 600)
        window.contentView = webView
        window.center()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)

        loadInterface()
    }

    private func loadInterface() {
        guard let resources = Bundle.main.resourceURL else {
            showError("Duxo could not start", "Application resources are missing.")
            return
        }

        let uiDirectory = resources.appendingPathComponent("ui", isDirectory: true)
        let index = uiDirectory.appendingPathComponent("index.html")
        guard FileManager.default.fileExists(atPath: index.path) else {
            showError("Duxo could not start", "The packaged UI is missing: \(index.path)")
            return
        }

        webView.loadFileURL(index, allowingReadAccessTo: uiDirectory)
    }

    func handleRequest(id: String, method: String, args: Any?) {
        switch method {
        case "getAppInfo":
            respond(id: id, result: ["name": "Duxo", "version": version, "platform": "darwin", "arch": "arm64"])
        case "prepareStartup": prepareStartup(id: id)
        case "pickGameFolder":
            let panel = NSOpenPanel(); panel.title = "Select a game folder"; panel.canChooseFiles = false; panel.canChooseDirectories = true; panel.allowsMultipleSelection = false
            if panel.runModal() == .OK { respond(id: id, result: panel.url?.path as Any) } else { respond(id: id, result: NSNull()) }
        case "analyzeGame":
            guard let gamePath = args as? String, !gamePath.isEmpty else { respondError(id: id, message: "A game directory is required."); return }
            analyzeGame(id: id, gamePath: gamePath)
        case "buildAdaptationPlan": buildAdaptationPlan(id: id, profile: args)
        default: respondError(id: id, message: "Unknown Duxo method: \(method)")
        }
    }

    private func prepareStartup(id: String) {
        guard let resources = Bundle.main.resourceURL else { respondError(id: id, message: "Application resources are missing."); return }
        let workspace = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support/Duxo/workspace", isDirectory: true)
        let analyzer = resources.appendingPathComponent("duxo-runtime/duxo-analyzer")
        let adapter = resources.appendingPathComponent("duxo-runtime/duxo-adapter")
        let stages: [(String, () throws -> Bool)] = [
            ("Checking application environment", { true }),
            ("Preparing local workspace", { try FileManager.default.createDirectory(at: workspace, withIntermediateDirectories: true); return FileManager.default.fileExists(atPath: workspace.path) }),
            ("Verifying Duxo Analyzer", { FileManager.default.isExecutableFile(atPath: analyzer.path) }),
            ("Verifying Duxo Adapter", { FileManager.default.isExecutableFile(atPath: adapter.path) }),
            ("Checking target platform", { true }),
            ("Finalizing local services", { true })
        ]
        do {
            for (index, stage) in stages.enumerated() {
                let ok = try stage.1()
                guard ok else { respondError(id: id, message: "\(stage.0) failed."); return }
                let percent = Int(((index + 1) * 100) / stages.count); sendProgress(message: stage.0, percent: percent, complete: percent == 100)
            }
            respond(id: id, result: ["workspace": workspace.path, "analyzer": analyzer.path, "adapter": adapter.path])
        } catch { respondError(id: id, message: error.localizedDescription) }
    }

    private func analyzeGame(id: String, gamePath: String) {
        DispatchQueue.global(qos: .userInitiated).async {
            guard let resources = Bundle.main.resourceURL else { DispatchQueue.main.async { self.respondError(id: id, message: "Application resources are missing.") }; return }
            let analyzer = resources.appendingPathComponent("duxo-runtime/duxo-analyzer")
            let process = Process(); let stdout = Pipe(); let stderr = Pipe()
            process.executableURL = analyzer; process.arguments = [gamePath]; process.standardOutput = stdout; process.standardError = stderr
            do {
                try process.run(); process.waitUntilExit()
                let output = stdout.fileHandleForReading.readDataToEndOfFile(); let errorOutput = stderr.fileHandleForReading.readDataToEndOfFile()
                guard process.terminationStatus == 0 else { let message = String(data: errorOutput, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "Analyzer exited with code \(process.terminationStatus)."; DispatchQueue.main.async { self.respondError(id: id, message: message) }; return }
                let json = try JSONSerialization.jsonObject(with: output, options: []); DispatchQueue.main.async { self.respond(id: id, result: json) }
            } catch { DispatchQueue.main.async { self.respondError(id: id, message: error.localizedDescription) } }
        }
    }

    private func buildAdaptationPlan(id: String, profile: Any?) {
        guard let profile, JSONSerialization.isValidJSONObject(profile) else { respondError(id: id, message: "A valid game profile is required to build an adaptation plan."); return }
        DispatchQueue.global(qos: .userInitiated).async {
            guard let resources = Bundle.main.resourceURL else { DispatchQueue.main.async { self.respondError(id: id, message: "Application resources are missing.") }; return }
            let adapter = resources.appendingPathComponent("duxo-runtime/duxo-adapter")
            guard FileManager.default.isExecutableFile(atPath: adapter.path) else { DispatchQueue.main.async { self.respondError(id: id, message: "Duxo Adapter is missing or not executable: \(adapter.path)") }; return }
            let process = Process(); let stdin = Pipe(); let stdout = Pipe(); let stderr = Pipe()
            process.executableURL = adapter; process.standardInput = stdin; process.standardOutput = stdout; process.standardError = stderr
            do {
                let input = try JSONSerialization.data(withJSONObject: profile, options: []); try process.run(); stdin.fileHandleForWriting.write(input); stdin.fileHandleForWriting.closeFile(); process.waitUntilExit()
                let output = stdout.fileHandleForReading.readDataToEndOfFile(); let errorOutput = stderr.fileHandleForReading.readDataToEndOfFile()
                guard process.terminationStatus == 0 else { let message = String(data: errorOutput, encoding: .utf8)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? "Adapter exited with code \(process.terminationStatus)."; DispatchQueue.main.async { self.respondError(id: id, message: message) }; return }
                let json = try JSONSerialization.jsonObject(with: output, options: []); DispatchQueue.main.async { self.respond(id: id, result: json) }
            } catch { DispatchQueue.main.async { self.respondError(id: id, message: error.localizedDescription) } }
        }
    }

    private func respond(id: String, result: Any) { DispatchQueue.main.async { guard let data = try? JSONSerialization.data(withJSONObject: result, options: []), let json = String(data: data, encoding: .utf8) else { self.respondError(id: id, message: "Failed to serialize native response."); return }; self.evaluate("window.__duxoResolve(\(Self.jsonString(id)), \(json));") } }
    private func respondError(id: String, message: String) { DispatchQueue.main.async { self.evaluate("window.__duxoReject(\(Self.jsonString(id)), \(Self.jsonString(message)));") } }
    private func sendProgress(message: String, percent: Int, complete: Bool) { evaluate("window.__duxoProgress(\(Self.jsonString(message)), \(percent), \(complete ? "true" : "false"));") }
    private func evaluate(_ script: String) { webView.evaluateJavaScript(script, completionHandler: nil) }
    private func showError(_ title: String, _ message: String) { let alert = NSAlert(); alert.alertStyle = .critical; alert.messageText = title; alert.informativeText = message; alert.runModal() }
    private static func jsonString(_ value: String) -> String { let data = try! JSONSerialization.data(withJSONObject: [value], options: []); let encoded = String(data: data, encoding: .utf8)!; return String(encoded.dropFirst().dropLast()) }

    private static let bridgeJavaScript = #"""
    (() => {
      const pending = new Map(); let sequence = 0;
      window.__duxoResolve = (id, value) => { const item = pending.get(id); if (!item) return; pending.delete(id); item.resolve(value); };
      window.__duxoReject = (id, message) => { const item = pending.get(id); if (!item) return; pending.delete(id); item.reject(new Error(message)); };
      window.__duxoProgress = (message, percent, complete) => { for (const listener of window.__duxoProgressListeners ?? []) listener({ message, percent, complete }); };
      window.__duxoProgressListeners = [];
      const request = (method, args) => new Promise((resolve, reject) => { const id = `native-${++sequence}`; pending.set(id, { resolve, reject }); window.webkit.messageHandlers.duxo.postMessage({ type: 'request', id, method, args }); });
      window.duxo = {
        getAppInfo: () => request('getAppInfo'), prepareStartup: () => request('prepareStartup'),
        onStartupProgress: (listener) => { window.__duxoProgressListeners.push(listener); return () => { window.__duxoProgressListeners = window.__duxoProgressListeners.filter((item) => item !== listener); }; },
        pickGameFolder: () => request('pickGameFolder'), analyzeGame: (gamePath) => request('analyzeGame', gamePath), buildAdaptationPlan: (profile) => request('buildAdaptationPlan', profile),
      };
    })();
    """#
}

let app = NSApplication.shared
let delegate = DuxoController()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()

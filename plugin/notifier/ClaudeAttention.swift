// ClaudeAttention — posts one macOS notification under this app bundle's identity, so it carries the
// bundle's icon (macOS takes a notification's icon from the sending app, nothing else).
// Usage: ClaudeAttention <title> <message> [group] [status-file]
// Exit / status-file value: 0 posted, 2 bad usage, 3 not authorized, 4 post failed.
// Launched directly before the user has allowed it, macOS denies authorization without a prompt;
// bin/notifier.sh then launches it through LaunchServices so it registers with Notification Center
// (where the user can allow it, if macOS still does not prompt).
import Foundation
import UserNotifications

let args = CommandLine.arguments
let statusFile = args.count > 4 ? args[4] : nil

func finish(_ code: Int32) -> Never {
    if let path = statusFile {
        try? "\(code)\n".write(toFile: path, atomically: true, encoding: .utf8)
    }
    exit(code)
}

guard args.count >= 3, !args[1].isEmpty else {
    FileHandle.standardError.write(Data("usage: ClaudeAttention <title> <message> [group] [status-file]\n".utf8))
    finish(2)
}
let center = UNUserNotificationCenter.current()
let done = DispatchSemaphore(value: 0)
var status: Int32 = 0
center.requestAuthorization(options: [.alert]) { granted, authError in
    if let e = authError { FileHandle.standardError.write(Data("authorization: \(e)\n".utf8)) }
    guard granted else { status = 3; done.signal(); return }
    let content = UNMutableNotificationContent()
    content.title = args[1]
    content.body = args[2]
    if args.count > 3, !args[3].isEmpty { content.threadIdentifier = args[3] }
    let request = UNNotificationRequest(identifier: UUID().uuidString, content: content, trigger: nil)
    center.add(request) { error in
        if error != nil { status = 4 }
        done.signal()
    }
}
if done.wait(timeout: .now() + 60) == .timedOut { status = 3 }
finish(status)

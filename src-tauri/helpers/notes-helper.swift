import Foundation

func runAppleScript(_ source: String) -> (result: String?, error: String?) {
    var errorDict: NSDictionary?
    guard let script = NSAppleScript(source: source) else {
        return (nil, "Failed to create NSAppleScript")
    }
    let descriptor = script.executeAndReturnError(&errorDict)
    if let errInfo = errorDict {
        let msg = (errInfo["NSAppleScriptErrorMessage"] as? String) ?? "Unknown AppleScript error"
        return (nil, msg)
    }
    return (descriptor.stringValue, nil)
}

func writeTmp(_ name: String, _ content: String) -> String? {
    let path = NSTemporaryDirectory() + name
    do {
        try content.write(toFile: path, atomically: true, encoding: .utf8)
        return path
    } catch {
        return nil
    }
}

let args = CommandLine.arguments
let FOLDER = "Produtividade de Bolso"
let command = args.count > 1 ? args[1] : ""

switch command {

case "ensure-folder":
    let folder = args.count > 2 ? args[2] : FOLDER
    let script = """
    tell application "Notes"
        if not (exists folder "\(folder)") then
            make new folder with properties {name: "\(folder)"}
        end if
    end tell
    """
    let (_, err) = runAppleScript(script)
    if let err = err { fputs("error: \(err)\n", stderr); exit(1) }
    print("ok")

case "list":
    let folder = args.count > 2 ? args[2] : FOLDER
    // Use ASCII separators to safely pass id/name pairs without JSON escaping in AppleScript
    let script = """
    tell application "Notes"
        set output to ""
        try
            set theFolder to folder "\(folder)"
            set noteList to notes of theFolder
            repeat with n in noteList
                set noteName to name of n as string
                if noteName is not "" then
                    set output to output & (id of n as string) & (ASCII character 0) & noteName & (ASCII character 1)
                end if
            end repeat
        end try
        return output
    end tell
    """
    let (result, err) = runAppleScript(script)
    if let err = err { fputs("error: \(err)\n", stderr); exit(1) }

    var notes: [[String: String]] = []
    if let raw = result, !raw.isEmpty {
        for record in raw.split(separator: "\u{01}", omittingEmptySubsequences: true) {
            let parts = record.split(separator: "\u{00}", maxSplits: 1)
            if parts.count == 2 {
                notes.append(["id": String(parts[0]), "name": String(parts[1])])
            }
        }
    }
    if let data = try? JSONSerialization.data(withJSONObject: notes),
       let json = String(data: data, encoding: .utf8) {
        print(json)
    } else {
        print("[]")
    }

case "get":
    guard args.count > 2 else { fputs("error: missing id\n", stderr); exit(1) }
    let noteId = args[2]
    let script = """
    tell application "Notes"
        return plaintext of (note id "\(noteId)")
    end tell
    """
    let (result, err) = runAppleScript(script)
    if let err = err { fputs("error: \(err)\n", stderr); exit(1) }

    // Body format in Mac Notes: "Title\n\nContent" — return everything after the title line
    let fullText = result ?? ""
    if let range = fullText.range(of: "\n\n") {
        print(String(fullText[range.upperBound...]))
    } else if let range = fullText.range(of: "\n") {
        print(String(fullText[range.upperBound...]))
    } else {
        print("") // title only, no content
    }

case "create":
    guard args.count > 3 else { fputs("error: missing folder/title\n", stderr); exit(1) }
    let folder = args[2]
    let title = args[3]
    // Write title to temp file to avoid AppleScript string quoting issues
    guard let tmpFile = writeTmp("notes_create.txt", title) else {
        fputs("error: failed to write temp file\n", stderr); exit(1)
    }
    let script = """
    tell application "Notes"
        set titleStr to read POSIX file "\(tmpFile)" as «class utf8»
        set newNote to make new note at folder "\(folder)" with properties {body: titleStr}
        return id of newNote as string
    end tell
    """
    let (result, err) = runAppleScript(script)
    if let err = err { fputs("error: \(err)\n", stderr); exit(1) }
    print(result ?? "")

case "update":
    guard args.count > 3 else { fputs("error: missing id/file\n", stderr); exit(1) }
    let noteId = args[2]
    let tmpFile = args[3]
    let script = """
    tell application "Notes"
        set n to note id "\(noteId)"
        set fileContent to read POSIX file "\(tmpFile)" as «class utf8»
        set body of n to fileContent
    end tell
    """
    let (_, err) = runAppleScript(script)
    if let err = err { fputs("error: \(err)\n", stderr); exit(1) }
    print("ok")

case "rename":
    guard args.count > 3 else { fputs("error: missing id/name-file\n", stderr); exit(1) }
    let noteId = args[2]
    let nameTmpFile = args[3]

    guard let newNameRaw = try? String(contentsOfFile: nameTmpFile, encoding: .utf8) else {
        fputs("error: failed to read name file\n", stderr); exit(1)
    }
    let newName = newNameRaw.trimmingCharacters(in: .newlines)

    // Get current plaintext from Notes
    let getScript = """
    tell application "Notes"
        return plaintext of (note id "\(noteId)")
    end tell
    """
    let (currentText, err1) = runAppleScript(getScript)
    if let err1 = err1 { fputs("error: \(err1)\n", stderr); exit(1) }

    // Replace the first line (title) with the new name in Swift — no AppleScript escaping needed
    var lines = (currentText ?? "").components(separatedBy: "\n")
    if !lines.isEmpty { lines[0] = newName }
    let newBody = lines.joined(separator: "\n")

    guard let bodyTmpFile = writeTmp("notes_rename.txt", newBody) else {
        fputs("error: failed to write body temp file\n", stderr); exit(1)
    }
    let setScript = """
    tell application "Notes"
        set n to note id "\(noteId)"
        set fileContent to read POSIX file "\(bodyTmpFile)" as «class utf8»
        set body of n to fileContent
    end tell
    """
    let (_, err2) = runAppleScript(setScript)
    if let err2 = err2 { fputs("error: \(err2)\n", stderr); exit(1) }
    print("ok")

case "delete":
    guard args.count > 2 else { fputs("error: missing id\n", stderr); exit(1) }
    let noteId = args[2]
    let script = """
    tell application "Notes"
        delete (note id "\(noteId)")
    end tell
    """
    let (_, err) = runAppleScript(script)
    if let err = err { fputs("error: \(err)\n", stderr); exit(1) }
    print("ok")

default:
    fputs("error: unknown command '\(command)'\n", stderr)
    exit(1)
}

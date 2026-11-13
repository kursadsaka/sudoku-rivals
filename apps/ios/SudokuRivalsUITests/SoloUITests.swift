import XCTest

/// Drives the real app on a simulator through accessibility identifiers from
/// docs/ui-contract.json and reads state back from cell labels.
final class SoloUITests: XCTestCase {
    // Seeds whose puzzles the singles hints finish (checked in SoloSessionTests).
    let seed = "7"
    var app: XCUIApplication!

    override func setUp() { continueAfterFailure = false }

    @discardableResult
    func launch(reset: Bool = false, language: String = "en", _ env: [String: String] = [:]) -> XCUIApplication {
        app = XCUIApplication()
        app.launchArguments = ["-AppleLanguages", "(\(language))", "-AppleLocale", language == "tr" ? "tr_TR" : "en_US"]
        app.launchEnvironment = env.merging(["SOLO_TEST_SEED": seed]) { a, _ in a }
        if reset { app.launchEnvironment["SOLO_TEST_RESET"] = "1" }
        app.launch()
        return app
    }

    func el(_ id: String) -> XCUIElement { app.descendants(matching: .any)[id].firstMatch }
    func cell(_ row: Int, _ column: Int) -> XCUIElement { app.buttons["cell-\(row)-\(column)"] }
    func cells(where predicate: String) -> XCUIElementQuery {
        app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'cell-' AND \(predicate)"))
    }
    /// "Row 1, column 2, editable, 5, notes: none" -> ("5", "none")
    func state(_ element: XCUIElement) -> (value: String, notes: String) {
        let parts = element.label.components(separatedBy: ", ")
        return (parts[3], parts[4].replacingOccurrences(of: "notes: ", with: ""))
    }
    func tap(_ id: String) {
        let element = el(id)
        XCTAssertTrue(element.waitForExistence(timeout: 5), "missing \(id)")
        element.tap()
    }

    func startGame(size: Int, confirm: Bool = false) {
        tap("solo-start")
        XCTAssertTrue(el("difficulty-note").waitForExistence(timeout: 5))
        XCTAssertFalse(el("game-board").exists) // no board on setup
        app.segmentedControls["board-size"].buttons["\(size)×\(size)"].tap()
        tap("game-start")
        if confirm { tap("confirm-replace") }
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 10))
        XCTAssertEqual(cells(where: "TRUEPREDICATE").count, size * size)
    }

    func firstEditableEmpty() -> XCUIElement {
        let element = cells(where: "label CONTAINS 'editable, empty'").firstMatch
        XCTAssertTrue(element.exists)
        return element
    }

    // MARK: Full play

    func testSixAndNineCompleteWithHintsAndShowResult() {
        for size in [6, 9] {
            launch(reset: true)
            startGame(size: size)
            XCTAssertTrue(el("number-\(size)").exists)
            XCTAssertFalse(el("number-\(size + 1)").exists)
            var moves = 0
            while !el("result-heading").exists {
                tap("hint")
                XCTAssertTrue(el("hint-explanation").waitForExistence(timeout: 5), "singles stalled")
                tap("hint-apply")
                moves += 1
                XCTAssertLessThanOrEqual(moves, size == 6 ? 18 : 43)
            }
            XCTAssertEqual(moves, size == 6 ? 18 : 43)
            XCTAssertEqual(el("game-status").label, "Sudoku complete")
            XCTAssertTrue(el("result-details").label.hasPrefix("\(size)×\(size)"))
            XCTAssertEqual(el("storage-status").label, "Saved on this device")
            add(screenshot("result-\(size)"))
            tap("home")
            XCTAssertTrue(el("solo-resume-\(size)").waitForExistence(timeout: 5))
            XCTAssertTrue(el("solo-resume-\(size)").label.contains("Completed"))
            // Completed result is consistent after a cold restart.
            app.terminate()
            launch()
            tap("solo-resume-\(size)")
            XCTAssertTrue(el("result-heading").waitForExistence(timeout: 5))
        }
    }

    func testGivenProtectionNotesEraseUndoRedo() {
        launch(reset: true)
        startGame(size: 6)
        add(screenshot("game-6"))
        let given = cells(where: "label CONTAINS ', given, '").firstMatch
        let givenLabel = given.label
        given.tap()
        XCTAssertFalse(el("number-1").isEnabled)
        XCTAssertFalse(el("erase").isEnabled)
        XCTAssertEqual(given.label, givenLabel)

        let target = firstEditableEmpty()
        let id = target.identifier
        target.tap()
        XCTAssertTrue(app.buttons[id].isSelected)
        tap("notes-toggle")
        tap("number-1"); tap("number-3")
        XCTAssertEqual(state(app.buttons[id]).notes, "1 3")
        tap("notes-toggle")
        tap("number-2")
        XCTAssertEqual(state(app.buttons[id]).value, "2")
        XCTAssertEqual(state(app.buttons[id]).notes, "none")
        tap("undo")
        XCTAssertEqual(state(app.buttons[id]).value, "empty")
        XCTAssertEqual(state(app.buttons[id]).notes, "1 3")
        tap("redo")
        XCTAssertEqual(state(app.buttons[id]).value, "2")
        tap("erase")
        XCTAssertEqual(state(app.buttons[id]).value, "empty")
        XCTAssertEqual(state(app.buttons[id]).notes, "none")
        XCTAssertFalse(el("redo").isEnabled)
        tap("undo")
        XCTAssertEqual(state(app.buttons[id]).value, "2")
    }

    func testVisibleConflictIsLabelledAndHintRefuses() {
        launch(reset: true)
        startGame(size: 6)
        // Put a row's given digit into an empty cell of that row.
        // Capture identifiers: label-based queries re-resolve after the board changes.
        let target = firstEditableEmpty().identifier
        let row = Int(target.split(separator: "-")[1])!
        let givenCell = cells(where: "identifier BEGINSWITH 'cell-\(row)-' AND label CONTAINS ', given, '").firstMatch
        let given = givenCell.identifier
        let digit = state(givenCell).value
        app.buttons[target].tap()
        tap("number-\(digit)")
        XCTAssertEqual(state(app.buttons[target]).value, digit) // conflicting digit is kept
        XCTAssertTrue(app.buttons[target].label.hasSuffix(", conflict"))
        XCTAssertTrue(app.buttons[given].label.hasSuffix(", conflict"))
        XCTAssertEqual(el("game-status").label, "The board has conflicts")
        tap("hint")
        XCTAssertEqual(el("game-notice").label, "Resolve the board conflicts before asking for a hint.")
        XCTAssertFalse(el("hint-apply").exists)
        add(screenshot("conflict-6"))
    }

    func testHintIsAdvisoryThenAppliedAndUndone() {
        launch(reset: true)
        startGame(size: 9)
        tap("hint")
        XCTAssertTrue(el("hint-explanation").waitForExistence(timeout: 5))
        let selected = cells(where: "selected == true").firstMatch
        let id = selected.identifier
        XCTAssertEqual(state(selected).value, "empty") // advice did not write anything
        add(screenshot("hint-9"))
        tap("hint-apply")
        XCTAssertNotEqual(state(app.buttons[id]).value, "empty")
        XCTAssertFalse(el("hint-apply").exists)
        tap("undo")
        XCTAssertEqual(state(app.buttons[id]).value, "empty")
    }

    // MARK: Lifecycle and storage

    func testColdRestoreKeepsBoardAndNotes() {
        launch(reset: true)
        startGame(size: 9)
        let a = firstEditableEmpty().identifier
        app.buttons[a].tap(); tap("number-4")
        let b = firstEditableEmpty().identifier
        app.buttons[b].tap(); tap("notes-toggle"); tap("number-7"); tap("number-9")
        app.terminate()

        launch()
        tap("solo-resume-9")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 5))
        XCTAssertEqual(state(app.buttons[a]).value, "4")
        XCTAssertEqual(state(app.buttons[b]).notes, "7 9")
        XCTAssertTrue(app.buttons[b].isSelected)
        XCTAssertEqual(el("notes-toggle").value as? String, "1")
    }

    func testPauseAndBackgroundStopTimerAndRemoveBoard() {
        launch(reset: true)
        startGame(size: 6)
        tap("pause")
        XCTAssertTrue(el("resume").waitForExistence(timeout: 5))
        XCTAssertFalse(el("game-board").exists)
        XCTAssertEqual(cells(where: "TRUEPREDICATE").count, 0)
        tap("resume")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 5))
        sleep(2)

        XCUIDevice.shared.press(.home)
        sleep(1)
        app.activate()
        XCTAssertTrue(el("resume").waitForExistence(timeout: 5))
        XCTAssertFalse(el("game-board").exists)
        let frozen = el("timer").label
        sleep(3)
        XCTAssertEqual(el("timer").label, frozen)
        add(screenshot("paused-6"))
        tap("resume")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 5))
    }

    func testCorruptSaveIsKeptUntilExplicitNewGame() {
        launch(reset: true, ["SOLO_TEST_CORRUPT": "9", "SOLO_TEST_FUTURE": "6"])
        XCTAssertTrue(el("storage-status").waitForExistence(timeout: 5))
        let problems = app.descendants(matching: .any).matching(identifier: "storage-status")
        XCTAssertEqual(problems.count, 2)
        XCTAssertFalse(el("solo-resume-9").exists)
        add(screenshot("corrupt-home"))
        // Not silently reset across launches.
        app.terminate()
        launch()
        XCTAssertEqual(app.descendants(matching: .any).matching(identifier: "storage-status").count, 2)

        tap("solo-start")
        app.segmentedControls["board-size"].buttons["9×9"].tap()
        XCTAssertTrue(el("replace-note").exists)
        tap("game-start")
        let cancel = app.alerts.buttons["Cancel"]
        XCTAssertTrue(cancel.waitForExistence(timeout: 5))
        cancel.tap()
        XCTAssertFalse(el("game-board").exists)
        tap("game-start")
        tap("confirm-replace")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 10))
        tap("back")
        XCTAssertTrue(el("solo-resume-9").waitForExistence(timeout: 5))
        // The future-version 6×6 save is still reported, untouched.
        XCTAssertEqual(app.descendants(matching: .any).matching(identifier: "storage-status").count, 1)
    }

    func testFailedWriteKeepsPreviousSaveAndBlocksFalseExit() {
        launch(reset: true)
        startGame(size: 6)
        let a = firstEditableEmpty().identifier
        app.buttons[a].tap(); tap("number-1")
        app.terminate()

        launch(["SOLO_TEST_FAIL_WRITES": "1"])
        tap("solo-resume-6")
        let b = firstEditableEmpty().identifier
        app.buttons[b].tap(); tap("number-2")
        XCTAssertEqual(state(app.buttons[b]).value, "2") // live board keeps the move
        XCTAssertTrue(el("retry").waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["Saved on this device"].exists)
        tap("retry")
        XCTAssertTrue(el("retry").exists)
        add(screenshot("save-error"))
        tap("pause")
        tap("save-exit")
        XCTAssertTrue(el("resume").exists, "must not leave as if saved")
        tap("discard-exit")
        tap("confirm-discard")
        XCTAssertTrue(el("solo-resume-6").waitForExistence(timeout: 5))
        app.terminate()

        launch()
        tap("solo-resume-6")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 5))
        XCTAssertEqual(state(app.buttons[a]).value, "1")
        XCTAssertEqual(state(app.buttons[b]).value, "empty")
    }

    func testSizesAreIndependent() {
        launch(reset: true)
        startGame(size: 6)
        let a = firstEditableEmpty().identifier
        app.buttons[a].tap(); tap("number-5")
        tap("back")
        startGame(size: 9) // no replacement prompt: the 9×9 slot is empty
        XCTAssertFalse(app.alerts.firstMatch.exists)
        tap("back")
        XCTAssertTrue(el("solo-resume-6").waitForExistence(timeout: 5))
        XCTAssertTrue(el("solo-resume-9").exists)
        tap("solo-resume-6")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 5))
        XCTAssertEqual(state(app.buttons[a]).value, "5")
        tap("back")
        // Replacing an existing size asks first.
        tap("solo-start")
        app.segmentedControls["board-size"].buttons["6×6"].tap()
        tap("game-start")
        XCTAssertTrue(el("confirm-replace").waitForExistence(timeout: 5))
    }

    // MARK: Input and presentation

    func testHardwareKeyboard() {
        launch(reset: true)
        startGame(size: 6)
        let target = firstEditableEmpty()
        let id = target.identifier
        target.tap()
        app.typeKey("n", modifierFlags: [])
        XCTAssertEqual(el("notes-toggle").value as? String, "1")
        app.typeKey("3", modifierFlags: [])
        XCTAssertEqual(state(app.buttons[id]).notes, "3")
        app.typeKey("n", modifierFlags: [])
        app.typeKey("4", modifierFlags: [])
        XCTAssertEqual(state(app.buttons[id]).value, "4")
        app.typeKey("z", modifierFlags: .command)
        XCTAssertEqual(state(app.buttons[id]).value, "empty")
        app.typeKey("z", modifierFlags: [.command, .shift])
        XCTAssertEqual(state(app.buttons[id]).value, "4")
        // XCUITest on the iOS 27.2 simulator never delivers Delete/Backspace presses
        // (key code 42 never reaches the app), so Delete is checked by hand instead.
        let parts = id.split(separator: "-")
        let (row, column) = (Int(parts[1])!, Int(parts[2])!)
        let key = row < 6 ? XCUIKeyboardKey.downArrow : XCUIKeyboardKey.upArrow
        app.typeKey(key, modifierFlags: [])
        XCTAssertTrue(cell(row < 6 ? row + 1 : row - 1, column).isSelected)
    }

    func testTurkishLabelsAndLargeText() {
        app = XCUIApplication()
        app.launchArguments = ["-AppleLanguages", "(tr)", "-AppleLocale", "tr_TR",
                               "-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXL"]
        app.launchEnvironment = ["SOLO_TEST_RESET": "1", "SOLO_TEST_SEED": seed]
        app.launch()
        XCTAssertTrue(app.buttons["solo-start"].label.contains("Tek başına oyna"))
        tap("solo-start")
        app.segmentedControls["board-size"].buttons["9×9"].tap()
        tap("game-start")
        XCTAssertTrue(el("game-board").waitForExistence(timeout: 10))
        XCTAssertTrue(cell(1, 1).label.hasPrefix("Satır 1, sütun 1, "))
        XCTAssertEqual(el("number-9").label, "9 yaz")
        add(screenshot("tr-large-9"))
        // Controls stay reachable by scrolling at accessibility sizes.
        let hint = el("hint")
        app.swipeUp(); app.swipeUp()
        XCTAssertTrue(hint.isHittable)
    }

    func screenshot(_ name: String) -> XCTAttachment {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        return attachment
    }
}

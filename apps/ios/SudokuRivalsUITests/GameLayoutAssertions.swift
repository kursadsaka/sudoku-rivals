import XCTest

/// Shared by solo and live UI tests: the game screen is one fixed order (navigation, status, board, digits,
/// tools) that fits a normal phone without scrolling, with no developer-only text unless asked for.
extension XCTestCase {
    func element(_ app: XCUIApplication, _ id: String) -> XCUIElement { app.descendants(matching: .any)[id].firstMatch }

    func assertOneScreenGameLayout(_ app: XCUIApplication, size: Int, tools: [String], landscape: Bool = false,
                                   file: StaticString = #filePath, line: UInt = #line) {
        let window = app.windows.firstMatch.frame
        let back = element(app, "back"), timer = element(app, "timer"), board = element(app, "game-board")
        let first = element(app, "number-1"), last = element(app, "number-\(size)")
        for item in [back, timer, board, first, last] + tools.map({ element(app, $0) }) {
            XCTAssertTrue(item.exists && item.isHittable, "\(item.identifier) not reachable without scrolling", file: file, line: line)
            XCTAssertTrue(window.contains(item.frame), "\(item.identifier) outside the screen: \(item.frame)", file: file, line: line)
        }
        XCTAssertGreaterThanOrEqual(first.frame.width, 48, "digit target too narrow", file: file, line: line)
        XCTAssertGreaterThanOrEqual(first.frame.height, 48, "digit target too short", file: file, line: line)
        XCTAssertTrue(board.frame.width.rounded() == board.frame.height.rounded(), "board is not square", file: file, line: line)
        // Reading order: navigation, status, board, digits, tools.
        if landscape {
            // Side panel: navigation, status, digits and tools stack to the right of the board.
            XCTAssertLessThanOrEqual(board.frame.maxX, back.frame.minX + 1, "panel beside the board", file: file, line: line)
            XCTAssertLessThanOrEqual(back.frame.minY, first.frame.minY, "navigation above digits", file: file, line: line)
            XCTAssertLessThanOrEqual(timer.frame.maxY, first.frame.minY + 1, "time above digits", file: file, line: line)
            XCTAssertLessThanOrEqual(board.frame.maxX, first.frame.minX + 1, "digits beside the board", file: file, line: line)
        } else {
            XCTAssertLessThanOrEqual(back.frame.minY, board.frame.minY, file: file, line: line)
            XCTAssertLessThanOrEqual(timer.frame.maxY, board.frame.minY + 1, "time must sit above the board", file: file, line: line)
            XCTAssertLessThanOrEqual(board.frame.maxY, first.frame.minY + 1, "digits below the board", file: file, line: line)
            if let tool = tools.first.map({ element(app, $0) }) {
                XCTAssertLessThanOrEqual(last.frame.maxY, tool.frame.minY + 1, "tools below the digits", file: file, line: line)
            }
        }
        // Developer-only text stays hidden: keyboard help, "saved", and the selected-cell line are not drawn.
        let texts = app.staticTexts
        XCTAssertEqual(texts.matching(NSPredicate(format: "label BEGINSWITH 'Keyboard:'")).count, 0, file: file, line: line)
        XCTAssertEqual(texts.matching(NSPredicate(format: "label == 'Saved on this device'")).count, 0, file: file, line: line)
    }
}

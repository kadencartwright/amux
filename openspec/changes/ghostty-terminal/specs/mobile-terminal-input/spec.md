# mobile-terminal-input Specification

## Purpose

Enable mobile terminal interaction via a unified keyboard area with modifier buttons and text input.

## ADDED Requirements

### Requirement: Unified keyboard area on mobile

The system SHALL display a unified keyboard area at the bottom of the terminal on mobile browsers.

#### Scenario: Mobile keyboard area visible

- **WHEN** a user accesses the terminal on a mobile browser (iOS Safari or Android Chrome)
- **THEN** a unified keyboard area is displayed below the terminal surface
- **AND** the keyboard area includes a modifier row and text input

### Requirement: Modifier button row

The system SHALL provide modifier buttons for Ctrl, Esc, Tab, arrow keys, and Enter.

#### Scenario: Modifier buttons available

- **WHEN** the mobile keyboard area is displayed
- **THEN** buttons for Ctrl, Esc, Tab, Up, Down, Left, Right, and Enter are available
- **AND** pressing a modifier latches it for the next input action

#### Scenario: Modifier applies to typed input

- **WHEN** the user taps Ctrl and then types "c"
- **THEN** the system sends "\x03" (Ctrl+C) to the terminal
- **AND** the modifier is cleared after use

### Requirement: Text input area

The system SHALL provide a text input area for typing terminal commands on mobile.

#### Scenario: Text input receives virtual keyboard

- **WHEN** the user taps on the text input area
- **THEN** the mobile virtual keyboard appears
- **AND** typed characters are captured in the input area

#### Scenario: Submit sends to terminal

- **WHEN** the user types text and presses Enter (or taps Send)
- **THEN** the text is sent to the terminal
- **AND** the input area is cleared

### Requirement: Desktop direct keyboard capture

The system SHALL capture keyboard input directly on the terminal for desktop browsers.

#### Scenario: Desktop captures keys directly

- **WHEN** a user focuses the terminal on a desktop browser
- **THEN** keyboard events are captured by ghostty-web
- **AND** keystrokes are sent directly to the terminal

#### Scenario: No visible input field on desktop

- **WHEN** a user accesses the terminal on a desktop browser
- **THEN** no text input field or modifier buttons are displayed
- **AND** the terminal surface receives focus automatically on session select

### Requirement: Mobile modifier latching

The system SHALL support latched modifiers for mobile input.

#### Scenario: Ctrl modifier latched

- **WHEN** the user taps the Ctrl button
- **THEN** the modifier is latched (visual indicator shows active)
- **AND** the next character typed is sent with Ctrl applied

#### Scenario: Modifier clears after use

- **WHEN** a latched modifier has been applied to a character
- **THEN** the modifier is cleared
- **AND** subsequent characters are sent without the modifier

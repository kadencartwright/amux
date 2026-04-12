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

The system SHALL provide a hidden or visually minimal text input area for mobile virtual keyboard access.

#### Scenario: Text input receives virtual keyboard

- **WHEN** the user taps on the mobile keyboard area
- **THEN** the mobile virtual keyboard appears
- **AND** typed characters are captured through the mobile input area

#### Scenario: Typed input forwards immediately

- **WHEN** the user types or pastes text through the mobile input area
- **THEN** the system forwards the resulting bytes to the terminal immediately without waiting for a submit action
- **AND** the mobile input area is cleared or normalized as needed for the next edit event

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

### Requirement: Mobile orientation and paste remain coherent

The system SHALL preserve coherent terminal interaction across orientation changes and paste actions on supported mobile browsers.

#### Scenario: Orientation change preserves terminal interaction

- **WHEN** a mobile user rotates the device during an active terminal session
- **THEN** terminal output remains coherent
- **AND** the mobile input area remains usable after the resize/orientation change

#### Scenario: Paste preserves terminal input order

- **WHEN** a mobile user pastes text into the mobile input area
- **THEN** the pasted bytes are forwarded to the terminal in the intended order
- **AND** the terminal state does not become corrupted by duplicate or reordered mobile input handling

### Requirement: Mobile input reliability threshold

The system SHALL meet an explicit reliability threshold for the required mobile key set.

#### Scenario: Scripted mobile reliability run

- **WHEN** a 5,000-key scripted input run is executed on iOS Safari or Android Chrome across text entry, modifiers, arrows, and Enter
- **THEN** at least 99.9% of intended key events are delivered to the terminal session

; KeyScroll - AutoHotkey v2 prototype
; Ctrl+Up/Down: continuous scroll while held
; Compile: Ahk2Exe.exe /in src-ahk/keyscroll.ahk /out keyscroll.exe
;
; Uses GetKeyState(, "P") to check physical key state.
; "P" = physical state (actual hardware), not logical state.
; This is reliable even while Send() is firing.

#Requires AutoHotkey v2.0
#SingleInstance Force

^Up:: {
    while (GetKeyState("Up", "P") && GetKeyState("Ctrl", "P")) {
        Send("{WheelUp}")
        Sleep(50)
    }
}

^Down:: {
    while (GetKeyState("Down", "P") && GetKeyState("Ctrl", "P")) {
        Send("{WheelDown}")
        Sleep(50)
    }
}
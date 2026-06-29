; KeyScroll - AutoHotkey v2 prototype
; Ctrl+Up/Down: continuous scroll while held
; Compile: Ahk2Exe.exe /in src-ahk/keyscroll.ahk /out keyscroll.exe

#Requires AutoHotkey v2.0
#SingleInstance Force

; Scroll state flags
scrollingUp := false
scrollingDown := false

; Register hotkeys
^Up:: {
    global scrollingUp
    scrollingUp := true
    scrollingDown := false
    SetTimer(ScrollUp, 50)
}

^Down:: {
    global scrollingDown
    scrollingDown := true
    scrollingUp := false
    SetTimer(ScrollDown, 50)
}

; Key release detection via KeyWait
^Up Up:: {
    global scrollingUp
    scrollingUp := false
    SetTimer(ScrollUp, 0)
}

^Down Up:: {
    global scrollingDown
    scrollingDown := false
    SetTimer(ScrollDown, 0)
}

ScrollUp() {
    global scrollingUp
    if (!scrollingUp) {
        SetTimer(ScrollUp, 0)
        return
    }
    Send("{WheelUp}")
}

ScrollDown() {
    global scrollingDown
    if (!scrollingDown) {
        SetTimer(ScrollDown, 0)
        return
    }
    Send("{WheelDown}")
}
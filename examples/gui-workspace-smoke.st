include smalltalk/gui/Bootstrap.st
!
W := World new initializeWidth: 128 height: 88 depth: 1.
S := WorkspaceWindow new initialize.
O := Point new setX: 4 y: 4.
C := Point new setX: 124 y: 84.
R := Rectangle new setOrigin: O corner: C.
S bounds: R.
S appendLine: '| x |'.
S appendLine: 'x := 1 + 2.'.
S appendLine: 'x * 3'.
W addSubview: S.
W render.
W hostDisplay savePNG: 'gui-workspace-smoke.png'
!

include smalltalk/gui/Bootstrap.st
!
W := World new initializeWidth: 192 height: 112 depth: 1.
B := BrowserWindow new initialize.
O := Point new setX: 4 y: 4.
C := Point new setX: 188 y: 108.
R := Rectangle new setOrigin: O corner: C.
Classes := Array new: 4.
Classes at: 1 put: 'POINT'.
Classes at: 2 put: 'RECTANGLE'.
Classes at: 3 put: 'FORM'.
Classes at: 4 put: 'WORLD'.
Methods := Array new: 4.
Methods at: 1 put: 'X'.
Methods at: 2 put: 'Y'.
Methods at: 3 put: 'SETX:Y:'.
Methods at: 4 put: 'DRAWON:'.
Source := Array new: 4.
Source at: 1 put: 'DRAWON: F'.
Source at: 2 put: '  F FILLRECT:'.
Source at: 3 put: '    BOUNDS W: 1'.
Source at: 4 put: '  ^ SELF'.
B bounds: R.
B classList: Classes.
B methodList: Methods.
B sourceLines: Source.
W addSubview: B.
W render.
W hostDisplay savePNG: 'gui-browser-smoke.png'
!

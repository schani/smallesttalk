include smalltalk/gui/Bootstrap.st
!
W := World new initializeWidth: 64 height: 48 depth: 1.
V := SystemWindow new initialize.
O := Point new setX: 8 y: 8.
C := Point new setX: 56 y: 40.
R := Rectangle new setOrigin: O corner: C.
V bounds: R.
W addSubview: V.
W render.
W hostDisplay savePNG: 'gui-headless-smoke.png'
!

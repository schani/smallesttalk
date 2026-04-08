include smalltalk/gui/Bootstrap.st
!
W := World new initializeWidth: 128 height: 72 depth: 1.
T := TranscriptWindow new initialize.
O := Point new setX: 4 y: 4.
C := Point new setX: 124 y: 68.
R := Rectangle new setOrigin: O corner: C.
T bounds: R.
T show: 'VM READY'.
T show: 'GUI SNAPSHOT OK'.
T show: 'NEXT WORKSPACE'.
W addSubview: T.
W render.
W hostDisplay savePNG: 'gui-transcript-smoke.png'
!

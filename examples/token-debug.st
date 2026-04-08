S := CoreScanner new.
S resetTo: 'x: value x := value'.
P0 := S position.
T1 := S scanToken.
P1 := S position.
T2 := S scanToken.
P2 := S position.
T3 := S scanToken.
P3 := S position.
T4 := S scanToken.
P4 := S position.
T5 := S scanToken.
P5 := S position.
!

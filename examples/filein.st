Behavior subclass: #Point instanceVariableNames: 'x y'
!
Point methodsFor: 'accessing'
!
x
    ^ x
!
x: value
    x := value
!
y
    ^ y
!
y: value
    y := value
!
P := Point new.
P x: 3.
P y: 4.
!

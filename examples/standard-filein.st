Behavior subclass: #StdPoint instanceVariableNames: 'x y' classVariableNames: '' poolDictionaries: '' category: 'Demo'
!
StdPoint methodsFor: 'accessing'
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
P := StdPoint new yourself.
P x: 30.
P y: 12.
!

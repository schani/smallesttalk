Total := 0.
1 to: 5 do: [:i | Total := Total + i].
Choice := true ifTrue: [ 'yes' ] ifFalse: [ 'no' ].
Missing := nil ifNil: [ 42 ] ifNotNil: [:x | x ].
!

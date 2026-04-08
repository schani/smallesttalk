pub const SOURCE: &str = r#"
Behavior methodsFor: 'testing'
!
isNil
    ^ false
!
notNil
    ^ true
!
ifNil: aBlock
    ^ self
!
ifNotNil: aBlock
    ^ aBlock value: self
!
ifNil: nilBlock ifNotNil: notNilBlock
    ^ notNilBlock value: self
!
~= other
    ^ (self = other) not
!
True methodsFor: 'controlling'
!
ifTrue: aBlock
    ^ aBlock value
!
ifFalse: aBlock
    ^ nil
!
ifTrue: trueBlock ifFalse: falseBlock
    ^ trueBlock value
!
ifFalse: falseBlock ifTrue: trueBlock
    ^ trueBlock value
!
and: aBlock
    ^ aBlock value
!
or: aBlock
    ^ true
!
not
    ^ false
!
False methodsFor: 'controlling'
!
ifTrue: aBlock
    ^ nil
!
ifFalse: aBlock
    ^ aBlock value
!
ifTrue: trueBlock ifFalse: falseBlock
    ^ falseBlock value
!
ifFalse: falseBlock ifTrue: trueBlock
    ^ falseBlock value
!
and: aBlock
    ^ false
!
or: aBlock
    ^ aBlock value
!
not
    ^ true
!
UndefinedObject methodsFor: 'testing'
!
isNil
    ^ true
!
notNil
    ^ false
!
ifNil: aBlock
    ^ aBlock value
!
ifNotNil: aBlock
    ^ nil
!
ifNil: nilBlock ifNotNil: notNilBlock
    ^ nilBlock value
!
BlockClosure methodsFor: 'controlling'
!
whileTrue: aBlock
    self value ifTrue: [ aBlock value. ^ self whileTrue: aBlock ].
    ^ nil
!
whileFalse: aBlock
    self value ifFalse: [ aBlock value. ^ self whileFalse: aBlock ].
    ^ nil
!
String methodsFor: 'copying'
!
, aString
    | out offset |
    out := String new: self size + aString size.
    1 to: self size do: [:i | out at: i put: (self at: i)].
    offset := self size.
    1 to: aString size do: [:i | out at: offset + i put: (aString at: i)].
    ^ out
!
String methodsFor: 'comparing'
!
= other
    self size = other size ifFalse: [ ^ false ].
    ^ self equalsString: other at: 1
!
~= other
    ^ (self = other) not
!
equalsString: other at: index
    index > self size ifTrue: [ ^ true ].
    (self at: index) = (other at: index) ifFalse: [ ^ false ].
    ^ self equalsString: other at: index + 1
!
SmallInteger methodsFor: 'iterating'
!
to: limit do: aBlock
    self <= limit ifTrue: [
        aBlock value: self.
        (self + 1) to: limit do: aBlock
    ].
    ^ self
!
timesRepeat: aBlock
    1 to: self do: [:ignored | aBlock value].
    ^ self
!
"#;

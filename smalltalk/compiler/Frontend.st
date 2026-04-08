Behavior subclass: #CoreToken instanceVariableNames: 'kind value start stop' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Frontend'
!
Behavior subclass: #CoreTokenStream instanceVariableNames: 'source position current' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Frontend'
!
Behavior subclass: #CoreScanner instanceVariableNames: 'source position stream' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Frontend'
!
Behavior subclass: #CoreParser instanceVariableNames: 'scanner currentToken lastNode' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Frontend'
!
CoreToken methodsFor: 'accessing'
!
kind
    ^ kind
!
kind: anObject
    kind := anObject
!
value
    ^ value
!
value: anObject
    value := anObject
!
start
    ^ start
!
start: anObject
    start := anObject
!
stop
    ^ stop
!
stop: anObject
    stop := anObject
!
CoreToken methodsFor: 'testing'
!
isIdentifier
    ^ kind = 1
!
isKeyword
    ^ kind = 2
!
isBinary
    ^ kind = 3
!
isLiteral
    ^ kind = 4
!
isEnd
    ^ kind = 99
!
CoreTokenStream methodsFor: 'accessing'
!
source
    ^ source
!
source: anObject
    source := anObject
!
position
    ^ position
!
position: anObject
    position := anObject
!
current
    ^ current
!
current: anObject
    current := anObject
!
CoreTokenStream methodsFor: 'testing'
!
atEnd
    ^ current isNil ifTrue: [ true ] ifFalse: [ current isEnd ]
!
CoreScanner methodsFor: 'accessing'
!
source
    ^ source
!
source: anObject
    source := anObject
!
position
    ^ position
!
position: anObject
    position := anObject
!
stream
    ^ stream
!
stream: anObject
    stream := anObject
!
CoreScanner methodsFor: 'scanning'
!
scan: aSourceString
    | token tokenStream |
    source := aSourceString.
    position := 1.
    token := CoreToken new.
    token kind: 99.
    token value: nil.
    token start: 1.
    token stop: 1.
    tokenStream := CoreTokenStream new.
    tokenStream source: aSourceString.
    tokenStream position: 1.
    tokenStream current: token.
    stream := tokenStream.
    ^ tokenStream
!
CoreParser methodsFor: 'accessing'
!
scanner
    scanner isNil ifTrue: [ scanner := CoreScanner new ].
    ^ scanner
!
scanner: anObject
    scanner := anObject
!
currentToken
    ^ currentToken
!
currentToken: anObject
    currentToken := anObject
!
lastNode
    ^ lastNode
!
lastNode: anObject
    lastNode := anObject
!
CoreParser methodsFor: 'parsing'
!
parseMethodSource: aSourceString
    | method tokenStream |
    tokenStream := self scanner scan: aSourceString.
    method := CoreMethodNode new.
    method source: aSourceString.
    method tokenStream: tokenStream.
    method arguments: nil.
    method temporaries: nil.
    method statements: nil.
    method beMethod.
    lastNode := method.
    ^ method
!
parseDoItSource: aSourceString
    | method tokenStream |
    tokenStream := self scanner scan: aSourceString.
    method := CoreMethodNode new.
    method source: aSourceString.
    method tokenStream: tokenStream.
    method arguments: nil.
    method temporaries: nil.
    method statements: nil.
    method beDoIt.
    lastNode := method.
    ^ method
!
parseExpressionSource: aSourceString
    | expression |
    expression := CoreExpressionNode new.
    expression source: aSourceString.
    expression receiver: nil.
    expression selector: nil.
    expression arguments: nil.
    expression value: aSourceString.
    expression beLiteral.
    lastNode := expression.
    ^ expression
!

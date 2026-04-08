Behavior subclass: #CoreListNode instanceVariableNames: 'value next' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Support'
!
Behavior subclass: #CoreSequenceBuilder instanceVariableNames: 'first last size' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Support'
!
CoreListNode methodsFor: 'accessing'
!
value
    ^ value
!
value: anObject
    value := anObject
!
next
    ^ next
!
next: anObject
    next := anObject
!
CoreSequenceBuilder methodsFor: 'initialization'
!
reset
    first := nil.
    last := nil.
    size := 0.
    ^ self
!
CoreSequenceBuilder methodsFor: 'accessing'
!
size
    ^ size ifNil: [ 0 ] ifNotNil: [:x | x ]
!
first
    ^ first
!
last
    ^ last
!
CoreSequenceBuilder methodsFor: 'adding'
!
add: anObject
    | node |
    node := CoreListNode new.
    node value: anObject.
    node next: nil.
    first isNil
        ifTrue: [ first := node. last := node ]
        ifFalse: [ last next: node. last := node ].
    size := self size + 1.
    ^ anObject
!
asArray
    | array |
    array := Array new: self size.
    self fillArray: array startingAt: 1 node: first.
    ^ array
!
fillArray: anArray startingAt: index node: aNode
    aNode isNil ifTrue: [ ^ anArray ].
    anArray at: index put: aNode value.
    ^ self fillArray: anArray startingAt: index + 1 node: aNode next
!
CoreToken methodsFor: 'testing-extended'
!
kindIs: anInteger
    ^ kind = anInteger
!
CoreScanner methodsFor: 'private'
!
resetTo: aSourceString
    source := aSourceString.
    position := 1.
    ^ self
!
atEnd
    ^ position > source size
!
peekByte
    self atEnd ifTrue: [ ^ nil ].
    ^ source at: position
!
peekNextByte
    position + 1 > source size ifTrue: [ ^ nil ].
    ^ source at: position + 1
!
advance
    | byte |
    byte := self peekByte.
    byte isNil ifFalse: [ position := position + 1 ].
    ^ byte
!
makeTokenKind: kindInteger value: aValue start: startIndex stop: stopIndex
    | token |
    token := CoreToken new.
    token kind: kindInteger.
    token value: aValue.
    token start: startIndex.
    token stop: stopIndex.
    ^ token
!
skipWhitespaceAndComments
    | byte |
    [
        byte := self peekByte.
        byte notNil and: [
            (byte = 32) or: [
            (byte = 9) or: [
            (byte = 10) or: [
            (byte = 13) or: [ byte = 34 ]]]]]
    ] whileTrue: [
        byte = 34
            ifTrue: [ self skipComment ]
            ifFalse: [ self advance ]
    ]
!
skipComment
    self advance.
    [ self peekByte notNil and: [ self advance ~= 34 ] ] whileTrue: [ nil ].
    ^ self
!
isIdentifierStart: byte
    | result |
    result := false.
    ((byte >= 65) and: [ byte <= 90 ]) ifTrue: [ result := true ].
    result ifFalse: [ ((byte >= 97) and: [ byte <= 122 ]) ifTrue: [ result := true ] ].
    result ifFalse: [ result := byte = 95 ].
    ^ result
!
isIdentifierContinue: byte
    | result |
    result := false.
    (self isIdentifierStart: byte) ifTrue: [ result := true ].
    result ifFalse: [ (byte >= 48) ifTrue: [ result := byte <= 57 ] ].
    ^ result
!
isBinaryByte: byte
    | result |
    result := false.
    byte = 33 ifTrue: [ result := true ].
    result ifFalse: [ byte = 37 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 38 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 42 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 43 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 44 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 45 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 47 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 60 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 61 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 62 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 63 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 64 ifTrue: [ result := true ] ].
    result ifFalse: [ byte = 92 ifTrue: [ result := true ] ].
    result ifFalse: [ result := byte = 126 ].
    ^ result
!
scanToken
    | byte start token |
    self skipWhitespaceAndComments.
    self atEnd ifTrue: [ token := self makeTokenKind: 99 value: nil start: position stop: position ].
    token isNil ifTrue: [
        byte := self peekByte.
        start := position.
        byte = 40 ifTrue: [ self advance. token := self makeTokenKind: 10 value: nil start: start stop: position ].
        token isNil ifTrue: [ byte = 41 ifTrue: [ self advance. token := self makeTokenKind: 11 value: nil start: start stop: position ] ].
        token isNil ifTrue: [ byte = 91 ifTrue: [ self advance. token := self makeTokenKind: 12 value: nil start: start stop: position ] ].
        token isNil ifTrue: [ byte = 93 ifTrue: [ self advance. token := self makeTokenKind: 13 value: nil start: start stop: position ] ].
        token isNil ifTrue: [ byte = 124 ifTrue: [ self advance. token := self makeTokenKind: 14 value: nil start: start stop: position ] ].
        token isNil ifTrue: [ byte = 46 ifTrue: [ self advance. token := self makeTokenKind: 15 value: nil start: start stop: position ] ].
        token isNil ifTrue: [ byte = 59 ifTrue: [ self advance. token := self makeTokenKind: 16 value: nil start: start stop: position ] ].
        token isNil ifTrue: [ byte = 94 ifTrue: [ self advance. token := self makeTokenKind: 9 value: nil start: start stop: position ] ].
        token isNil ifTrue: [
            byte = 58 ifTrue: [
                self advance.
                self peekByte = 61
                    ifTrue: [ self advance. token := self makeTokenKind: 8 value: nil start: start stop: position ]
                    ifFalse: [ token := self makeTokenKind: 17 value: nil start: start stop: position ]
            ]
        ].
        token isNil ifTrue: [ byte = 35 ifTrue: [ token := self scanHashTokenStartingAt: start ] ].
        token isNil ifTrue: [ byte = 39 ifTrue: [ token := self scanStringStartingAt: start ] ].
        token isNil ifTrue: [ ((byte >= 48) and: [ byte <= 57 ]) ifTrue: [ token := self scanIntegerStartingAt: start ] ].
        token isNil ifTrue: [ (self isIdentifierStart: byte) ifTrue: [ token := self scanIdentifierOrKeywordStartingAt: start ] ].
        token isNil ifTrue: [ (self isBinaryByte: byte) ifTrue: [ token := self scanBinaryStartingAt: start ] ].
        token isNil ifTrue: [ self advance. token := self makeTokenKind: 99 value: nil start: start stop: position ]
    ].
    ^ token
!
scanIntegerStartingAt: start
    | value byte |
    value := 0.
    [
        byte := self peekByte.
        byte notNil and: [ (byte >= 48) and: [ byte <= 57 ] ]
    ] whileTrue: [
        value := value * 10 + (byte - 48).
        self advance
    ].
    ^ self makeTokenKind: 4 value: value start: start stop: position
!
scanIdentifierOrKeywordStartingAt: start
    | stop byte |
    [
        byte := self peekByte.
        byte notNil and: [ self isIdentifierContinue: byte ]
    ] whileTrue: [ self advance ].
    (self peekByte = 58 and: [ self peekNextByte ~= 61 ])
        ifTrue: [
            self advance.
            stop := position.
            ^ self makeTokenKind: 2 value: (source copyFrom: start to: stop - 1) start: start stop: stop
        ].
    stop := position.
    ^ self makeTokenKind: 1 value: (source copyFrom: start to: stop - 1) start: start stop: stop
!
scanBinaryStartingAt: start
    | byte |
    [
        byte := self peekByte.
        byte notNil and: [ self isBinaryByte: byte ]
    ] whileTrue: [ self advance ].
    ^ self makeTokenKind: 3 value: (source copyFrom: start to: position - 1) start: start stop: position
!
scanStringStartingAt: start
    | builder done byte charCode string |
    builder := CoreSequenceBuilder new reset.
    self advance.
    done := false.
    [ done not and: [ self peekByte notNil ] ] whileTrue: [
        byte := self advance.
        byte = 39
            ifTrue: [
                self peekByte = 39
                    ifTrue: [ self advance. builder add: 39 ]
                    ifFalse: [ done := true ]
            ]
            ifFalse: [ builder add: byte ]
    ].
    string := String new: builder size.
    self fillByteString: string fromBuilderArray: builder asArray startingAt: 1.
    ^ self makeTokenKind: 5 value: string start: start stop: position
!
fillByteString: aString fromBuilderArray: anArray startingAt: index
    index > anArray size ifTrue: [ ^ aString ].
    aString at: index put: (anArray at: index).
    ^ self fillByteString: aString fromBuilderArray: anArray startingAt: index + 1
!
scanHashTokenStartingAt: start
    | next token |
    self advance.
    self peekByte = 40 ifTrue: [ self advance. ^ self makeTokenKind: 7 value: nil start: start stop: position ].
    self peekByte = 39 ifTrue: [
        token := self scanStringStartingAt: start + 1.
        ^ self makeTokenKind: 6 value: (Behavior internSymbol: token value) start: start stop: token stop
    ].
    (self isIdentifierStart: self peekByte)
        ifTrue: [ token := self scanIdentifierOrKeywordStartingAt: position. ^ self makeTokenKind: 6 value: (Behavior internSymbol: token value) start: start stop: token stop ].
    ((self peekByte notNil) and: [ self isBinaryByte: self peekByte ])
        ifTrue: [ token := self scanBinaryStartingAt: position. ^ self makeTokenKind: 6 value: (Behavior internSymbol: token value) start: start stop: token stop ].
    next := self makeTokenKind: 99 value: nil start: start stop: position.
    ^ next
!
CoreParser methodsFor: 'private'
!
advance
    currentToken := self scanner scanToken.
    ^ currentToken
!
peekToken
    | saved token |
    saved := self scanner position.
    token := self scanner scanToken.
    self scanner position: saved.
    ^ token
!
expectKind: kindInteger
    (currentToken kindIs: kindInteger) ifFalse: [ ^ nil ].
    ^ self advance
!
makeMethodNodeWithSource: aString
    | node |
    node := CoreMethodNode new.
    node source: aString.
    node arguments: Array new: 0.
    node temporaries: Array new: 0.
    node statements: Array new: 0.
    ^ node
!
makeExpressionNodeWithValue: aValue
    | node |
    node := CoreExpressionNode new.
    node source: nil.
    node receiver: nil.
    node selector: nil.
    node arguments: Array new: 0.
    node value: aValue.
    node beLiteral.
    ^ node
!
parsePrimary
    | node value |
    (currentToken kindIs: 4) ifTrue: [
        value := currentToken value.
        self advance.
        ^ self makeExpressionNodeWithValue: value
    ].
    ((currentToken kindIs: 5) or: [ currentToken kindIs: 6 ]) ifTrue: [
        value := currentToken value.
        self advance.
        ^ self makeExpressionNodeWithValue: value
    ].
    (currentToken kindIs: 1) ifTrue: [
        value := currentToken value.
        self advance.
        node := self makeExpressionNodeWithValue: value.
        node beVariable.
        ^ node
    ].
    (currentToken kindIs: 10) ifTrue: [
        self advance.
        node := self parseExpression.
        self expectKind: 11.
        ^ node
    ].
    (currentToken kindIs: 12) ifTrue: [ ^ self parseBlock ].
    ^ self makeExpressionNodeWithValue: nil
!
parseUnaryExpression
    | receiver node builder |
    receiver := self parsePrimary.
    [ currentToken kindIs: 1 ] whileTrue: [
        node := CoreExpressionNode new.
        node receiver: receiver.
        node selector: currentToken value.
        node arguments: Array new: 0.
        node value: nil.
        node beSend.
        receiver := node.
        self advance
    ].
    ^ receiver
!
parseBinaryExpression
    | receiver node argument |
    receiver := self parseUnaryExpression.
    [ currentToken kindIs: 3 ] whileTrue: [
        node := CoreExpressionNode new.
        node receiver: receiver.
        node selector: currentToken value.
        self advance.
        argument := self parseUnaryExpression.
        node arguments: (self arrayWith: argument).
        node value: nil.
        node beSend.
        receiver := node
    ].
    ^ receiver
!
parseKeywordExpression
    | receiver node builder selector |
    receiver := self parseBinaryExpression.
    (currentToken kindIs: 2) ifFalse: [ ^ receiver ].
    builder := CoreSequenceBuilder new reset.
    selector := ''.
    [ currentToken kindIs: 2 ] whileTrue: [
        selector := selector , currentToken value.
        self advance.
        builder add: self parseBinaryExpression
    ].
    node := CoreExpressionNode new.
    node receiver: receiver.
    node selector: selector.
    node arguments: builder asArray.
    node value: nil.
    node beSend.
    ^ node
!
parseExpression
    ^ self parseKeywordExpression
!
parseReturnStatement
    | returnNode |
    returnNode := nil.
    (currentToken kindIs: 9) ifTrue: [
        self advance.
        returnNode := CoreReturnNode new.
        returnNode expression: self parseExpression.
    ].
    ^ returnNode
!
parseAssignmentStatement
    | name assignment |
    assignment := nil.
    (currentToken kindIs: 1) ifTrue: [
        ((self peekToken) kindIs: 8) ifTrue: [
            name := currentToken value.
            self advance.
            self advance.
            assignment := CoreAssignmentNode new.
            assignment name: name.
            assignment expression: self parseExpression.
        ]
    ].
    ^ assignment
!
parseStatement
    | statement |
    statement := self parseReturnStatement.
    statement isNil ifTrue: [ statement := self parseAssignmentStatement ].
    statement isNil ifTrue: [ statement := self parseExpression ].
    ^ statement
!
makeSyntheticIdentifierToken: aString
    | token |
    token := CoreToken new.
    token kind: 1.
    token value: aString.
    token start: 0.
    token stop: 0.
    ^ token
!
parseStatementsUntilKind: stopKind
    | builder statement |
    builder := CoreSequenceBuilder new reset.
    [ (currentToken kindIs: stopKind) not and: [ currentToken isEnd not ] ] whileTrue: [
        statement := self parseStatement.
        builder add: statement.
        (currentToken kindIs: 15) ifTrue: [ self advance ]
    ].
    ^ builder asArray
!
parseBlock
    | block args temps builder |
    block := CoreBlockNode new.
    self advance.
    builder := CoreSequenceBuilder new reset.
    [ currentToken kindIs: 17 ] whileTrue: [
        self advance.
        (currentToken kindIs: 1) ifTrue: [ builder add: currentToken value. self advance ]
    ].
    (builder size > 0 and: [ currentToken kindIs: 14 ]) ifTrue: [ self advance ].
    args := builder asArray.
    temps := self parseTemporaries.
    block arguments: args.
    block temporaries: temps.
    block statements: (self parseStatementsUntilKind: 13).
    self expectKind: 13.
    ^ block
!
parseTemporaries
    | builder |
    (currentToken kindIs: 14) ifFalse: [ ^ Array new: 0 ].
    self advance.
    builder := CoreSequenceBuilder new reset.
    [ (currentToken kindIs: 14) not and: [ currentToken isEnd not ] ] whileTrue: [
        (currentToken kindIs: 1) ifTrue: [ builder add: currentToken value ].
        self advance
    ].
    self expectKind: 14.
    ^ builder asArray
!
parseMethodPatternInto: methodNode
    | selector builder |
    builder := CoreSequenceBuilder new reset.
    (currentToken kindIs: 1) ifTrue: [
        selector := currentToken value.
        self advance.
        (currentToken kindIs: 1)
            ifTrue: [ methodNode selector: selector. methodNode arguments: Array new: 0. ^ methodNode ].
        methodNode selector: selector.
        methodNode arguments: Array new: 0.
        ^ methodNode
    ].
    (currentToken kindIs: 3) ifTrue: [
        selector := currentToken value.
        self advance.
        (currentToken kindIs: 1) ifTrue: [ builder add: currentToken value. self advance ].
        methodNode selector: selector.
        methodNode arguments: builder asArray.
        ^ methodNode
    ].
    selector := ''.
    [ currentToken kindIs: 2 ] whileTrue: [
        selector := selector , currentToken value.
        self advance.
        (currentToken kindIs: 1) ifTrue: [ builder add: currentToken value. self advance ]
    ].
    methodNode selector: selector.
    methodNode arguments: builder asArray.
    ^ methodNode
!
arrayWith: anObject
    | array |
    array := Array new: 1.
    array at: 1 put: anObject.
    ^ array
!
CoreParser methodsFor: 'parsing-real'
!
parseMethodSource: aSourceString
    | method |
    self scanner resetTo: aSourceString.
    self advance.
    method := self makeMethodNodeWithSource: aSourceString.
    method beMethod.
    self parseMethodPatternInto: method.
    method temporaries: self parseTemporaries.
    method statements: (self parseStatementsUntilKind: 99).
    lastNode := method.
    ^ method
!
parseDoItSource: aSourceString
    | method |
    self scanner resetTo: aSourceString.
    self advance.
    method := self makeMethodNodeWithSource: aSourceString.
    method beDoIt.
    method selector: 'DoIt'.
    method arguments: Array new: 0.
    method temporaries: self parseTemporaries.
    method statements: (self parseStatementsUntilKind: 99).
    lastNode := method.
    ^ method
!
parseExpressionSource: aSourceString
    | expression |
    self scanner resetTo: aSourceString.
    self advance.
    expression := self parseExpression.
    lastNode := expression.
    ^ expression
!

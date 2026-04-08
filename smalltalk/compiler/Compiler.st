Behavior subclass: #CoreBytecodeBuilder instanceVariableNames: 'ast targetClass literals bytecodes argumentNames temporaryNames capturedNames activationKind' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Backend'
!
Behavior subclass: #CoreCompiledUnit instanceVariableNames: 'ast targetClass builder source kind compiledMethod' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Backend'
!
Behavior subclass: #CoreCompiler instanceVariableNames: 'parser builder lastAst lastUnit currentClass' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-Backend'
!
CoreBytecodeBuilder methodsFor: 'accessing'
!
ast
    ^ ast
!
ast: anObject
    ast := anObject
!
targetClass
    ^ targetClass
!
targetClass: anObject
    targetClass := anObject
!
literals
    ^ literals
!
literals: anObject
    literals := anObject
!
bytecodes
    ^ bytecodes
!
bytecodes: anObject
    bytecodes := anObject
!
argumentNames
    ^ argumentNames
!
argumentNames: anObject
    argumentNames := anObject
!
temporaryNames
    ^ temporaryNames
!
temporaryNames: anObject
    temporaryNames := anObject
!
capturedNames
    ^ capturedNames
!
capturedNames: anObject
    capturedNames := anObject
!
activationKind
    ^ activationKind
!
activationKind: anObject
    activationKind := anObject
!
CoreBytecodeBuilder methodsFor: 'building'
!
prepareFor: anAst inClass: aClass
    ast := anAst.
    targetClass := aClass.
    literals := CoreSequenceBuilder new reset.
    bytecodes := CoreSequenceBuilder new reset.
    argumentNames := Array new: 0.
    temporaryNames := Array new: 0.
    capturedNames := Array new: 0.
    activationKind := 1.
    ^ self
!
buildCompiledUnit
    ^ self buildCompiledUnitInstalling: targetClass notNil
!
buildCompiledUnitInstalling: shouldInstall
    | unit method |
    unit := CoreCompiledUnit new.
    unit ast: ast.
    unit targetClass: targetClass.
    unit builder: self.
    unit source: ast source.
    unit kind: ast kind.
    method := self compileMethodNode: ast forClass: targetClass captured: (Array new: 0) install: shouldInstall.
    unit compiledMethod: method.
    ^ unit
!
compileMethodNode: aMethodNode forClass: aClass captured: captured install: shouldInstall
    | method selector argCount tempCount literalsArray bytecodesArray |
    argumentNames := aMethodNode arguments.
    temporaryNames := aMethodNode temporaries.
    capturedNames := captured.
    literals := CoreSequenceBuilder new reset.
    bytecodes := CoreSequenceBuilder new reset.
    activationKind := 1.
    self emitBoxingPrologue.
    self compileStatements: aMethodNode statements.
    argCount := self totalArgumentCount.
    tempCount := self totalTemporaryCount.
    literalsArray := literals asArray.
    bytecodesArray := bytecodes asArray.
    shouldInstall
        ifTrue: [
            selector := aMethodNode selector.
            ^ aClass installMethod: selector literals: literalsArray bytecodes: bytecodesArray numArgs: argCount numTemps: tempCount
        ]
        ifFalse: [
            ^ Behavior compiledMethod: aMethodNode selector literals: literalsArray bytecodes: bytecodesArray numArgs: argCount numTemps: tempCount
        ]
!
compileStatements: statements
    self compileStatements: statements index: 1
!
compileStatements: statements index: index
    index > statements size ifTrue: [
        activationKind = 2
            ifTrue: [ self emitPushNil. self emitByte: 228 ]
            ifFalse: [ self emitByte: 226 ].
        ^ self
    ].
    self compileStatement: (statements at: index) isLast: index = statements size.
    ^ self compileStatements: statements index: index + 1
!
compileStatement: aStatement isLast: isLast
    (aStatement class name = #CoreAssignmentNode)
        ifTrue: [ ^ self compileAssignment: aStatement isLast: isLast ].
    (aStatement class name = #CoreReturnNode)
        ifTrue: [ self compileExpression: aStatement expression. self emitByte: 225. ^ self ].
    self compileExpression: aStatement.
    isLast
        ifTrue: [ activationKind = 2 ifTrue: [ self emitByte: 228 ] ifFalse: [ self emitByte: 225 ] ]
        ifFalse: [ self emitByte: 89 ]
!
compileAssignment: anAssignment isLast: isLast
    | localIndex ivarIndex assoc |
    localIndex := self localIndexOf: anAssignment name.
    localIndex notNil ifTrue: [
        self emitPushTemp: localIndex.
        self compileExpression: anAssignment expression.
        self emitSelectorSend: 'value:' super: false.
        isLast ifFalse: [ self emitByte: 89 ].
        ^ self
    ].
    ivarIndex := self instanceVariableIndexOf: anAssignment name.
    ivarIndex notNil ifTrue: [
        self compileExpression: anAssignment expression.
        isLast ifTrue: [ self emitByte: 88 ].
        self emitPopStoreInstVar: ivarIndex.
        ^ self
    ].
    assoc := Behavior globalAssociation: anAssignment name.
    self emitPushLiteralObject: assoc.
    self compileExpression: anAssignment expression.
    self emitSelectorSend: 'value:' super: false.
    isLast ifFalse: [ self emitByte: 89 ]
!
compileExpression: anExpression
    anExpression isNil ifTrue: [ ^ self emitPushNil ].
    (anExpression class name = #CoreBlockNode) ifTrue: [ ^ self compileBlock: anExpression ].
    (anExpression isLiteral) ifTrue: [ ^ self compileLiteral: anExpression value ].
    (anExpression isVariable) ifTrue: [ ^ self compileVariableNamed: anExpression value ].
    (anExpression isSend) ifTrue: [ ^ self compileSendNode: anExpression ].
    ^ self emitPushNil
!
compileLiteral: aValue
    | className |
    aValue isNil ifTrue: [ ^ self emitPushNil ].
    className := aValue class name.
    className = #True ifTrue: [ ^ self emitPushTrue ].
    className = #False ifTrue: [ ^ self emitPushFalse ].
    className = #SmallInteger ifTrue: [ ^ self emitSmallInteger: aValue ].
    ^ self emitPushLiteralObject: aValue
!
compileVariableNamed: aName
    | localIndex ivarIndex |
    aName = 'self' ifTrue: [ ^ self emitPushSelf ].
    aName = 'nil' ifTrue: [ ^ self emitPushNil ].
    aName = 'true' ifTrue: [ ^ self emitPushTrue ].
    aName = 'false' ifTrue: [ ^ self emitPushFalse ].
    aName = 'thisContext' ifTrue: [ self emitPushSelf. ^ self emitSelectorSend: 'thisContext' super: false ].
    localIndex := self localIndexOf: aName.
    localIndex notNil ifTrue: [ self emitPushTemp: localIndex. ^ self emitSelectorSend: 'value' super: false ].
    ivarIndex := self instanceVariableIndexOf: aName.
    ivarIndex notNil ifTrue: [ ^ self emitPushInstVar: ivarIndex ].
    ^ self emitPushLitVarAssociationNamed: aName
!
compileSendNode: aSend
    | receiver isSuper |
    receiver := aSend receiver.
    isSuper := false.
    (receiver class name = #CoreExpressionNode)
        ifTrue: [ isSuper := receiver isVariable and: [ receiver value = 'super' ] ].
    isSuper
        ifTrue: [ self emitPushSelf ]
        ifFalse: [ self compileExpression: receiver ].
    self compileExpressionArray: aSend arguments index: 1.
    ^ self emitSelectorSend: aSend selector super: isSuper
!
compileExpressionArray: expressions index: index
    index > expressions size ifTrue: [ ^ self ].
    self compileExpression: (expressions at: index).
    ^ self compileExpressionArray: expressions index: index + 1
!
compileBlock: aBlock
    | visible nested blockMethod literalIndex copiedCount |
    visible := self visibleNames.
    self emitCopiedTemps: visible index: 1.
    nested := CoreBytecodeBuilder new.
    nested prepareFor: ast inClass: targetClass.
    blockMethod := nested compileBlockNode: aBlock forClass: targetClass captured: visible.
    literalIndex := self addLiteral: blockMethod.
    copiedCount := visible size.
    self emitByte: 224.
    self emitByte: aBlock arguments size.
    self emitByte: copiedCount.
    self emitByte: 0.
    self emitByte: 0.
    ^ self emitByte: literalIndex
!
compileBlockNode: aBlock forClass: aClass captured: captured
    | pseudoMethod |
    pseudoMethod := CoreMethodNode new.
    pseudoMethod source: nil.
    pseudoMethod selector: '[]'.
    pseudoMethod arguments: aBlock arguments.
    pseudoMethod temporaries: aBlock temporaries.
    pseudoMethod statements: aBlock statements.
    pseudoMethod beDoIt.
    argumentNames := pseudoMethod arguments.
    temporaryNames := pseudoMethod temporaries.
    capturedNames := captured.
    literals := CoreSequenceBuilder new reset.
    bytecodes := CoreSequenceBuilder new reset.
    activationKind := 2.
    self emitBoxingPrologue.
    self compileStatements: pseudoMethod statements.
    ^ Behavior compiledMethod: '[]' literals: literals asArray bytecodes: bytecodes asArray numArgs: self totalArgumentCount numTemps: self totalTemporaryCount
!
emitCopiedTemps: names index: index
    index > names size ifTrue: [ ^ self ].
    self emitPushTemp: (index - 1).
    ^ self emitCopiedTemps: names index: index + 1
!
emitBoxingPrologue
    self emitBoxingForIndicesFrom: self capturedCount to: self capturedCount + argumentNames size - 1 withInitialValues: true.
    self emitBoxingForIndicesFrom: self capturedCount + argumentNames size to: self capturedCount + argumentNames size + temporaryNames size - 1 withInitialValues: false
!
emitBoxingForIndicesFrom: start to: stop withInitialValues: hasInitial
    start > stop ifTrue: [ ^ self ].
    self emitPushLiteralObject: Association.
    self emitSelectorSend: 'new' super: false.
    self emitByte: 88.
    hasInitial
        ifTrue: [ self emitPushTemp: start ]
        ifFalse: [ self emitPushNil ].
    self emitSelectorSend: 'value:' super: false.
    self emitByte: 89.
    self emitPopStoreTemp: start.
    ^ self emitBoxingForIndicesFrom: start + 1 to: stop withInitialValues: hasInitial
!
CoreBytecodeBuilder methodsFor: 'lookup'
!
capturedCount
    ^ capturedNames size
!
localIndexOf: aName
    | index |
    index := self indexOfName: aName in: argumentNames startingAt: self capturedCount.
    index notNil ifTrue: [ ^ index ].
    index := self indexOfName: aName in: temporaryNames startingAt: self capturedCount + argumentNames size.
    index notNil ifTrue: [ ^ index ].
    ^ self indexOfName: aName in: capturedNames startingAt: 0
!
indexOfName: aName in: array startingAt: offset
    ^ self indexOfName: aName in: array startingAt: offset index: 1
!
indexOfName: aName in: array startingAt: offset index: index
    index > array size ifTrue: [ ^ nil ].
    ((array at: index) = aName) ifTrue: [ ^ offset + index - 1 ].
    ^ self indexOfName: aName in: array startingAt: offset index: index + 1
!
instanceVariableIndexOf: aName
    | index |
    targetClass isNil ifTrue: [ ^ nil ].
    index := targetClass instanceVariableIndex: aName.
    index isNil ifTrue: [ ^ nil ].
    ^ index
!
visibleNames
    | builder |
    builder := CoreSequenceBuilder new reset.
    self addNamesFrom: capturedNames to: builder index: 1.
    self addNamesFrom: argumentNames to: builder index: 1.
    self addNamesFrom: temporaryNames to: builder index: 1.
    ^ builder asArray
!
addNamesFrom: names to: builder index: index
    index > names size ifTrue: [ ^ builder ].
    builder add: (names at: index).
    ^ self addNamesFrom: names to: builder index: index + 1
!
totalArgumentCount
    ^ self capturedCount + argumentNames size
!
totalTemporaryCount
    ^ temporaryNames size
!
CoreBytecodeBuilder methodsFor: 'emitting'
!
addLiteral: anObject
    literals add: anObject.
    ^ literals size - 1
!
emitByte: value
    bytecodes add: value.
    ^ value
!
emitPushSelf
    ^ self emitByte: 80
!
emitPushNil
    ^ self emitByte: 81
!
emitPushTrue
    ^ self emitByte: 82
!
emitPushFalse
    ^ self emitByte: 83
!
emitSmallInteger: value
    value = (0 - 1) ifTrue: [ ^ self emitByte: 84 ].
    value = 0 ifTrue: [ ^ self emitByte: 85 ].
    value = 1 ifTrue: [ ^ self emitByte: 86 ].
    value = 2 ifTrue: [ ^ self emitByte: 87 ].
    (value >= 0 and: [ value <= 255 ]) ifTrue: [ self emitByte: 141. ^ self emitByte: value ].
    ^ self emitPushLiteralObject: value
!
emitPushTemp: index
    index < 16 ifTrue: [ ^ self emitByte: 16 + index ].
    self emitByte: 129.
    ^ self emitByte: index
!
emitPopStoreTemp: index
    index < 8 ifTrue: [ ^ self emitByte: 72 + index ].
    self emitByte: 133.
    ^ self emitByte: index
!
emitPushInstVar: index
    index < 16 ifTrue: [ ^ self emitByte: index ].
    self emitByte: 128.
    ^ self emitByte: index
!
emitPopStoreInstVar: index
    index < 8 ifTrue: [ ^ self emitByte: 64 + index ].
    self emitByte: 132.
    ^ self emitByte: index
!
emitPushLiteralObject: anObject
    | index |
    index := self addLiteral: anObject.
    index < 16 ifTrue: [ ^ self emitByte: 32 + index ].
    self emitByte: 130.
    ^ self emitByte: index
!
emitPushLitVarAssociationNamed: aName
    | index assoc |
    assoc := Behavior globalAssociation: aName.
    index := self addLiteral: assoc.
    index < 16 ifTrue: [ ^ self emitByte: 48 + index ].
    self emitByte: 131.
    ^ self emitByte: index
!
emitSelectorSend: selectorString super: isSuper
    | special index |
    special := self specialSendIndexFor: selectorString.
    (isSuper not and: [ special notNil ]) ifTrue: [ ^ self emitByte: 112 + special ].
    index := self addLiteral: selectorString.
    index < 16
        ifTrue: [
            isSuper ifTrue: [ self emitByte: 135. ^ self emitByte: index ].
            ^ self emitByte: 96 + index
        ].
    self emitByte: (isSuper ifTrue: [ 135 ] ifFalse: [ 134 ]).
    ^ self emitByte: index
!
specialSendIndexFor: selectorString
    selectorString = '+' ifTrue: [ ^ 0 ].
    selectorString = '-' ifTrue: [ ^ 1 ].
    selectorString = '*' ifTrue: [ ^ 2 ].
    selectorString = '/' ifTrue: [ ^ 3 ].
    selectorString = '<' ifTrue: [ ^ 4 ].
    selectorString = '>' ifTrue: [ ^ 5 ].
    selectorString = '<=' ifTrue: [ ^ 6 ].
    selectorString = '>=' ifTrue: [ ^ 7 ].
    selectorString = '=' ifTrue: [ ^ 8 ].
    selectorString = '~=' ifTrue: [ ^ 9 ].
    selectorString = 'bitAnd:' ifTrue: [ ^ 10 ].
    selectorString = 'bitOr:' ifTrue: [ ^ 11 ].
    selectorString = 'bitShift:' ifTrue: [ ^ 12 ].
    selectorString = '@' ifTrue: [ ^ 13 ].
    selectorString = 'at:' ifTrue: [ ^ 14 ].
    selectorString = 'at:put:' ifTrue: [ ^ 15 ].
    ^ nil
!
CoreCompiledUnit methodsFor: 'accessing'
!
ast
    ^ ast
!
ast: anObject
    ast := anObject
!
targetClass
    ^ targetClass
!
targetClass: anObject
    targetClass := anObject
!
builder
    ^ builder
!
builder: anObject
    builder := anObject
!
source
    ^ source
!
source: anObject
    source := anObject
!
kind
    ^ kind
!
kind: anObject
    kind := anObject
!
compiledMethod
    ^ compiledMethod
!
compiledMethod: anObject
    compiledMethod := anObject
!
CoreCompiler methodsFor: 'accessing'
!
parser
    parser isNil ifTrue: [ parser := CoreParser new ].
    ^ parser
!
parser: anObject
    parser := anObject
!
builder
    builder isNil ifTrue: [ builder := CoreBytecodeBuilder new ].
    ^ builder
!
builder: anObject
    builder := anObject
!
lastAst
    ^ lastAst
!
lastAst: anObject
    lastAst := anObject
!
lastUnit
    ^ lastUnit
!
lastUnit: anObject
    lastUnit := anObject
!
currentClass
    ^ currentClass
!
currentClass: anObject
    currentClass := anObject
!
CoreCompiler methodsFor: 'frontend'
!
parseMethodSource: aSourceString
    lastAst := self parser parseMethodSource: aSourceString.
    ^ lastAst
!
parseDoItSource: aSourceString
    lastAst := self parser parseDoItSource: aSourceString.
    ^ lastAst
!
parseExpressionSource: aSourceString
    lastAst := self parser parseExpressionSource: aSourceString.
    ^ lastAst
!
CoreCompiler methodsFor: 'backend'
!
compileMethodSource: aSourceString forClass: aClass
    ^ self compileMethodSource: aSourceString forClass: aClass install: true
!
compileMethodSource: aSourceString forClass: aClass install: shouldInstall
    | parsed |
    currentClass := aClass.
    parsed := self parseMethodSource: aSourceString.
    lastUnit := (self builder prepareFor: parsed inClass: aClass) buildCompiledUnitInstalling: shouldInstall.
    ^ lastUnit
!
compileMethodSourceWithoutInstall: aSourceString forClass: aClass
    ^ self compileMethodSource: aSourceString forClass: aClass install: false
!
compileDoItSource: aSourceString
    | parsed |
    parsed := self parseDoItSource: aSourceString.
    lastUnit := (self builder prepareFor: parsed inClass: nil) buildCompiledUnit.
    ^ lastUnit
!
compileAndInstallMethodSource: aSourceString forClass: aClass
    ^ (self compileMethodSource: aSourceString forClass: aClass) compiledMethod
!
CoreCompiler methodsFor: 'introspection'
!
hasParsedNode
    ^ lastAst notNil
!
hasCompiledUnit
    ^ lastUnit notNil
!
reset
    parser := nil.
    builder := nil.
    lastAst := nil.
    lastUnit := nil.
    currentClass := nil.
    ^ self
!

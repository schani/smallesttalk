Behavior subclass: #CoreNode instanceVariableNames: 'source' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-AST'
!
CoreNode subclass: #CoreMethodNode instanceVariableNames: 'selector arguments temporaries statements tokenStream kind' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-AST'
!
CoreNode subclass: #CoreExpressionNode instanceVariableNames: 'receiver selector arguments value kind' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-AST'
!
CoreNode subclass: #CoreBlockNode instanceVariableNames: 'arguments temporaries statements' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-AST'
!
CoreNode subclass: #CoreAssignmentNode instanceVariableNames: 'name expression' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-AST'
!
CoreNode subclass: #CoreReturnNode instanceVariableNames: 'expression' classVariableNames: '' poolDictionaries: '' category: 'CoreCompiler-AST'
!
CoreNode methodsFor: 'accessing'
!
source
    ^ source
!
source: anObject
    source := anObject
!
CoreMethodNode methodsFor: 'accessing'
!
selector
    ^ selector
!
selector: anObject
    selector := anObject
!
arguments
    ^ arguments
!
arguments: anObject
    arguments := anObject
!
temporaries
    ^ temporaries
!
temporaries: anObject
    temporaries := anObject
!
statements
    ^ statements
!
statements: anObject
    statements := anObject
!
tokenStream
    ^ tokenStream
!
tokenStream: anObject
    tokenStream := anObject
!
kind
    ^ kind
!
kind: anObject
    kind := anObject
!
CoreMethodNode methodsFor: 'initialization'
!
beMethod
    kind := 1.
    ^ self
!
beDoIt
    kind := 2.
    selector := #DoIt.
    ^ self
!
CoreExpressionNode methodsFor: 'accessing'
!
receiver
    ^ receiver
!
receiver: anObject
    receiver := anObject
!
selector
    ^ selector
!
selector: anObject
    selector := anObject
!
arguments
    ^ arguments
!
arguments: anObject
    arguments := anObject
!
value
    ^ value
!
value: anObject
    value := anObject
!
kind
    ^ kind
!
kind: anObject
    kind := anObject
!
CoreExpressionNode methodsFor: 'testing'
!
isLiteral
    ^ kind = 1
!
isVariable
    ^ kind = 2
!
isSend
    ^ kind = 3
!
isBlock
    ^ kind = 4
!
beLiteral
    kind := 1.
    ^ self
!
beVariable
    kind := 2.
    ^ self
!
beSend
    kind := 3.
    ^ self
!
beBlock
    kind := 4.
    ^ self
!
CoreBlockNode methodsFor: 'accessing'
!
arguments
    ^ arguments
!
arguments: anObject
    arguments := anObject
!
temporaries
    ^ temporaries
!
temporaries: anObject
    temporaries := anObject
!
statements
    ^ statements
!
statements: anObject
    statements := anObject
!
CoreAssignmentNode methodsFor: 'accessing'
!
name
    ^ name
!
name: anObject
    name := anObject
!
expression
    ^ expression
!
expression: anObject
    expression := anObject
!
CoreReturnNode methodsFor: 'accessing'
!
expression
    ^ expression
!
expression: anObject
    expression := anObject
!

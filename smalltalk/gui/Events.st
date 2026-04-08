Behavior subclass: #GuiEvent instanceVariableNames: 'kind' classVariableNames: '' poolDictionaries: '' category: 'GUI-Events'
!
GuiEvent subclass: #MouseEvent instanceVariableNames: 'x y button' classVariableNames: '' poolDictionaries: '' category: 'GUI-Events'
!
GuiEvent subclass: #KeyboardEvent instanceVariableNames: 'key' classVariableNames: '' poolDictionaries: '' category: 'GUI-Events'
!
GuiEvent subclass: #ResizeEvent instanceVariableNames: 'width height' classVariableNames: '' poolDictionaries: '' category: 'GUI-Events'
!
GuiEvent subclass: #QuitEvent instanceVariableNames: '' classVariableNames: '' poolDictionaries: '' category: 'GUI-Events'
!
Behavior subclass: #InputSensor instanceVariableNames: '' classVariableNames: '' poolDictionaries: '' category: 'GUI-Events'
!
GuiEvent methodsFor: 'accessing'
!
kind
    ^ kind
!
kind: anObject
    kind := anObject
!
MouseEvent methodsFor: 'accessing'
!
x
    ^ x
!
x: anObject
    x := anObject
!
y
    ^ y
!
y: anObject
    y := anObject
!
button
    ^ button
!
button: anObject
    button := anObject
!
KeyboardEvent methodsFor: 'accessing'
!
key
    ^ key
!
key: anObject
    key := anObject
!
ResizeEvent methodsFor: 'accessing'
!
width
    ^ width
!
width: anObject
    width := anObject
!
height
    ^ height
!
height: anObject
    height := anObject
!
InputSensor methodsFor: 'events'
!
nextEvent
    | raw type event |
    raw := Behavior hostNextEvent.
    raw isNil ifTrue: [ ^ nil ].
    type := raw at: 1.
    type = #mouseMove ifTrue: [
        event := MouseEvent new.
        event kind: type.
        event x: (raw at: 2).
        event y: (raw at: 3).
        event button: 0.
        ^ event
    ].
    type = #mouseDown ifTrue: [
        event := MouseEvent new.
        event kind: type.
        event x: (raw at: 2).
        event y: (raw at: 3).
        event button: (raw at: 4).
        ^ event
    ].
    type = #mouseUp ifTrue: [
        event := MouseEvent new.
        event kind: type.
        event x: (raw at: 2).
        event y: (raw at: 3).
        event button: (raw at: 4).
        ^ event
    ].
    type = #keyDown ifTrue: [
        event := KeyboardEvent new.
        event kind: type.
        event key: (raw at: 2).
        ^ event
    ].
    type = #keyUp ifTrue: [
        event := KeyboardEvent new.
        event kind: type.
        event key: (raw at: 2).
        ^ event
    ].
    type = #resize ifTrue: [
        event := ResizeEvent new.
        event kind: type.
        event width: (raw at: 2).
        event height: (raw at: 3).
        ^ event
    ].
    type = #quit ifTrue: [
        event := QuitEvent new.
        event kind: type.
        ^ event
    ].
    event := GuiEvent new.
    event kind: type.
    ^ event
!

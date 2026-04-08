SystemWindow subclass: #TranscriptWindow instanceVariableNames: 'lines maxLines textFont lineHeight textInset' classVariableNames: '' poolDictionaries: '' category: 'GUI-Tools'
!
SystemWindow subclass: #WorkspaceWindow instanceVariableNames: 'lines textFont lineHeight textInset' classVariableNames: '' poolDictionaries: '' category: 'GUI-Tools'
!
TranscriptWindow methodsFor: 'initialization'
!
initialize
    super initialize.
    self title: 'TRANSCRIPT'.
    lines := Array new: 0.
    maxLines := 16.
    textFont := BitmapFont new initializeDefault.
    lineHeight := 8.
    textInset := 2.
    ^ self
!
TranscriptWindow methodsFor: 'accessing'
!
lines
    ^ lines
!
maxLines
    ^ maxLines
!
maxLines: anObject
    maxLines := anObject
!
textFont
    ^ textFont
!
textFont: anObject
    textFont := anObject
!
lineHeight
    ^ lineHeight
!
lineHeight: anObject
    lineHeight := anObject
!
textInset
    ^ textInset
!
textInset: anObject
    textInset := anObject
!
TranscriptWindow methodsFor: 'logging'
!
show: aString
    | oldSize newArray index sourceIndex |
    oldSize := lines size.
    oldSize < maxLines
        ifTrue: [
            newArray := Array new: oldSize + 1.
            index := 1.
            [ index <= oldSize ] whileTrue: [
                newArray at: index put: (lines at: index).
                index := index + 1
            ].
            newArray at: oldSize + 1 put: aString
        ]
        ifFalse: [
            newArray := Array new: maxLines.
            index := 1.
            sourceIndex := oldSize - maxLines + 2.
            [ index < maxLines ] whileTrue: [
                newArray at: index put: (lines at: sourceIndex).
                index := index + 1.
                sourceIndex := sourceIndex + 1
            ].
            newArray at: maxLines put: aString
        ].
    lines := newArray.
    ^ aString
!
clear
    lines := Array new: 0.
    ^ self
!
TranscriptWindow methodsFor: 'drawing'
!
drawOn: aForm
    super drawOn: aForm.
    self drawLinesOn: aForm.
    ^ self
!
drawLinesOn: aForm
    | startX startY index |
    bounds isNil ifTrue: [ ^ self ].
    startX := ((bounds origin) x) + textInset.
    startY := ((bounds origin) y) + titleBarHeight + 2.
    index := 1.
    [ index <= lines size ] whileTrue: [
        aForm drawString: (lines at: index) atX: startX y: startY font: textFont color: 1.
        startY := startY + lineHeight.
        index := index + 1
    ].
    ^ self
!
WorkspaceWindow methodsFor: 'initialization'
!
initialize
    super initialize.
    self title: 'WORKSPACE'.
    lines := Array new: 0.
    textFont := BitmapFont new initializeDefault.
    lineHeight := 8.
    textInset := 2.
    ^ self
!
WorkspaceWindow methodsFor: 'accessing'
!
lines
    ^ lines
!
setLines: anArray
    lines := anArray.
    ^ self
!
textFont
    ^ textFont
!
textFont: anObject
    textFont := anObject
!
lineHeight
    ^ lineHeight
!
lineHeight: anObject
    lineHeight := anObject
!
textInset
    ^ textInset
!
textInset: anObject
    textInset := anObject
!
WorkspaceWindow methodsFor: 'editing'
!
appendLine: aString
    | newArray index |
    newArray := Array new: lines size + 1.
    index := 1.
    [ index <= lines size ] whileTrue: [
        newArray at: index put: (lines at: index).
        index := index + 1
    ].
    newArray at: lines size + 1 put: aString.
    lines := newArray.
    ^ aString
!
clear
    lines := Array new: 0.
    ^ self
!
WorkspaceWindow methodsFor: 'drawing'
!
drawOn: aForm
    super drawOn: aForm.
    self drawLinesOn: aForm.
    ^ self
!
drawLinesOn: aForm
    | startX startY index |
    bounds isNil ifTrue: [ ^ self ].
    startX := ((bounds origin) x) + textInset.
    startY := ((bounds origin) y) + titleBarHeight + 2.
    index := 1.
    [ index <= lines size ] whileTrue: [
        aForm drawString: (lines at: index) atX: startX y: startY font: textFont color: 1.
        startY := startY + lineHeight.
        index := index + 1
    ].
    ^ self
!

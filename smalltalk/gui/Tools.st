SystemWindow subclass: #TranscriptWindow instanceVariableNames: 'lines maxLines textFont lineHeight textInset' classVariableNames: '' poolDictionaries: '' category: 'GUI-Tools'
!
SystemWindow subclass: #WorkspaceWindow instanceVariableNames: 'lines textFont lineHeight textInset' classVariableNames: '' poolDictionaries: '' category: 'GUI-Tools'
!
SystemWindow subclass: #BrowserWindow instanceVariableNames: 'classList methodList sourceLines textFont lineHeight textInset classPaneWidth methodPaneHeight' classVariableNames: '' poolDictionaries: '' category: 'GUI-Tools'
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
drawContentsOn: aCanvas
    super drawContentsOn: aCanvas.
    self drawLinesOn: (aCanvas clippedTo: self contentBounds).
    ^ self
!
drawLinesOn: aCanvas
    | startX startY index |
    (self contentBounds) isNil ifTrue: [ ^ self ].
    startX := ((self contentBounds) origin) x + textInset.
    startY := ((self contentBounds) origin) y + 1.
    index := 1.
    [ index <= lines size ] whileTrue: [
        aCanvas drawString: (lines at: index) atX: startX y: startY font: textFont color: 1.
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
drawContentsOn: aCanvas
    super drawContentsOn: aCanvas.
    self drawLinesOn: (aCanvas clippedTo: self contentBounds).
    ^ self
!
drawLinesOn: aCanvas
    | startX startY index |
    (self contentBounds) isNil ifTrue: [ ^ self ].
    startX := ((self contentBounds) origin) x + textInset.
    startY := ((self contentBounds) origin) y + 1.
    index := 1.
    [ index <= lines size ] whileTrue: [
        aCanvas drawString: (lines at: index) atX: startX y: startY font: textFont color: 1.
        startY := startY + lineHeight.
        index := index + 1
    ].
    ^ self
!
BrowserWindow methodsFor: 'initialization'
!
initialize
    super initialize.
    self title: 'BROWSER'.
    classList := Array new: 0.
    methodList := Array new: 0.
    sourceLines := Array new: 0.
    textFont := BitmapFont new initializeDefault.
    lineHeight := 8.
    textInset := 2.
    classPaneWidth := 48.
    methodPaneHeight := 28.
    ^ self
!
BrowserWindow methodsFor: 'accessing'
!
classList
    ^ classList
!
classList: anArray
    classList := anArray
!
methodList
    ^ methodList
!
methodList: anArray
    methodList := anArray
!
sourceLines
    ^ sourceLines
!
sourceLines: anArray
    sourceLines := anArray
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
classPaneWidth
    ^ classPaneWidth
!
classPaneWidth: anObject
    classPaneWidth := anObject
!
methodPaneHeight
    ^ methodPaneHeight
!
methodPaneHeight: anObject
    methodPaneHeight := anObject
!
BrowserWindow methodsFor: 'layout'
!
classesPaneBounds
    | content rect |
    content := self contentBounds.
    content isNil ifTrue: [ ^ nil ].
    rect := Rectangle new.
    rect
        setX: (content origin) x
        y: (content origin) y
        width: classPaneWidth
        height: content height.
    ^ rect
!
methodsPaneBounds
    | content rect |
    content := self contentBounds.
    content isNil ifTrue: [ ^ nil ].
    rect := Rectangle new.
    rect
        setX: (content origin) x + classPaneWidth - 1
        y: (content origin) y
        width: content width - classPaneWidth + 1
        height: methodPaneHeight.
    ^ rect
!
sourcePaneBounds
    | content rect |
    content := self contentBounds.
    content isNil ifTrue: [ ^ nil ].
    rect := Rectangle new.
    rect
        setX: (content origin) x + classPaneWidth - 1
        y: (content origin) y + methodPaneHeight - 1
        width: content width - classPaneWidth + 1
        height: content height - methodPaneHeight + 1.
    ^ rect
!
paneLabelBounds: aRectangle
    | rect |
    rect := Rectangle new.
    rect
        setX: (aRectangle origin) x + 1
        y: (aRectangle origin) y + 1
        width: aRectangle width - 2
        height: lineHeight.
    ^ rect
!
paneTextBounds: aRectangle
    | rect |
    rect := Rectangle new.
    rect
        setX: (aRectangle origin) x + 2
        y: (aRectangle origin) y + lineHeight + 2
        width: aRectangle width - 4
        height: aRectangle height - lineHeight - 3.
    ^ rect
!
BrowserWindow methodsFor: 'drawing'
!
drawContentsOn: aCanvas
    super drawContentsOn: aCanvas.
    self drawPanesOn: (aCanvas clippedTo: self contentBounds).
    ^ self
!
drawPanesOn: aCanvas
    | classesRect methodsRect sourceRect |
    (self contentBounds) isNil ifTrue: [ ^ self ].
    classesRect := self classesPaneBounds.
    methodsRect := self methodsPaneBounds.
    sourceRect := self sourcePaneBounds.
    aCanvas frameRectangleX: ((classesRect origin) x) y: ((classesRect origin) y) width: classesRect width height: classesRect height with: 1.
    aCanvas frameRectangleX: ((methodsRect origin) x) y: ((methodsRect origin) y) width: methodsRect width height: methodsRect height with: 1.
    aCanvas frameRectangleX: ((sourceRect origin) x) y: ((sourceRect origin) y) width: sourceRect width height: sourceRect height with: 1.
    self drawPaneLabel: 'CLASSES' in: classesRect on: aCanvas.
    self drawPaneLabel: 'METHODS' in: methodsRect on: aCanvas.
    self drawPaneLabel: 'SOURCE' in: sourceRect on: aCanvas.
    self drawList: classList in: (self paneTextBounds: classesRect) on: aCanvas.
    self drawList: methodList in: (self paneTextBounds: methodsRect) on: aCanvas.
    self drawList: sourceLines in: (self paneTextBounds: sourceRect) on: aCanvas.
    ^ self
!
drawPaneLabel: aString in: aRectangle on: aCanvas
    | labelRect |
    labelRect := self paneLabelBounds: aRectangle.
    aCanvas fillRectangle: labelRect with: 1.
    (aCanvas clippedTo: labelRect)
        drawString: aString
        atX: ((labelRect origin) x) + textInset - 1
        y: (labelRect origin) y
        font: textFont
        color: 0.
    ^ self
!
drawList: anArray in: aRectangle on: aCanvas
    | index drawY listCanvas |
    aRectangle width <= 0 ifTrue: [ ^ self ].
    aRectangle height <= 0 ifTrue: [ ^ self ].
    listCanvas := aCanvas clippedTo: aRectangle.
    index := 1.
    drawY := (aRectangle origin) y.
    [ index <= anArray size ] whileTrue: [
        listCanvas drawString: (anArray at: index) atX: (aRectangle origin) x y: drawY font: textFont color: 1.
        drawY := drawY + lineHeight.
        index := index + 1
    ].
    ^ self
!

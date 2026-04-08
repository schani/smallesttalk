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
BrowserWindow methodsFor: 'drawing'
!
drawOn: aForm
    super drawOn: aForm.
    self drawPanesOn: aForm.
    ^ self
!
drawPanesOn: aForm
    | x y w h contentX contentY contentW contentH leftW rightX rightW topH bottomY bottomH |
    bounds isNil ifTrue: [ ^ self ].
    x := (bounds origin) x.
    y := (bounds origin) y.
    w := bounds width.
    h := bounds height.
    contentX := x + 1.
    contentY := y + titleBarHeight + 1.
    contentW := w - 2.
    contentH := h - titleBarHeight - 2.
    leftW := classPaneWidth.
    rightX := contentX + leftW + 1.
    rightW := contentW - leftW - 1.
    topH := methodPaneHeight.
    bottomY := contentY + topH + 1.
    bottomH := contentH - topH - 1.
    aForm frameRectangleX: contentX y: contentY width: leftW height: contentH with: 1.
    aForm frameRectangleX: rightX y: contentY width: rightW height: topH with: 1.
    aForm frameRectangleX: rightX y: bottomY width: rightW height: bottomH with: 1.
    self drawPaneLabel: 'CLASSES' on: aForm atX: contentX y: contentY width: leftW.
    self drawPaneLabel: 'METHODS' on: aForm atX: rightX y: contentY width: rightW.
    self drawPaneLabel: 'SOURCE' on: aForm atX: rightX y: bottomY width: rightW.
    self drawList: classList on: aForm atX: contentX + 2 y: contentY + lineHeight + 2 width: leftW - 4 height: contentH - lineHeight - 3.
    self drawList: methodList on: aForm atX: rightX + 2 y: contentY + lineHeight + 2 width: rightW - 4 height: topH - lineHeight - 3.
    self drawList: sourceLines on: aForm atX: rightX + 2 y: bottomY + lineHeight + 2 width: rightW - 4 height: bottomH - lineHeight - 3.
    ^ self
!
drawPaneLabel: aString on: aForm atX: x y: y width: width
    aForm fillRectangleX: x + 1 y: y + 1 width: width - 2 height: lineHeight with: 1.
    aForm drawString: aString atX: x + textInset y: y + 1 font: textFont color: 0.
    ^ self
!
drawList: anArray on: aForm atX: startX y: startY width: width height: height
    | buffer index drawY |
    width <= 0 ifTrue: [ ^ self ].
    height <= 0 ifTrue: [ ^ self ].
    buffer := Form new initializeWidth: width height: height depth: 1.
    index := 1.
    drawY := 0.
    [ index <= anArray size ] whileTrue: [
        buffer drawString: (anArray at: index) atX: 0 y: drawY font: textFont color: 1.
        drawY := drawY + lineHeight.
        index := index + 1
    ].
    aForm copyRectangleX: startX y: startY width: width height: height from: buffer atX: 0 y: 0.
    ^ self
!

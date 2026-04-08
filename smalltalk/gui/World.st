Behavior subclass: #View instanceVariableNames: 'bounds subviews' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
!
View subclass: #SolidView instanceVariableNames: 'fillValue' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
!
View subclass: #LabelView instanceVariableNames: 'text font color' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
!
View subclass: #SystemWindow instanceVariableNames: 'fillValue borderValue titleBarValue titleBarHeight title titleColor titleFont titleInset' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
!
Behavior subclass: #World instanceVariableNames: 'hostDisplay displayForm rootView inputSensor' classVariableNames: '' poolDictionaries: '' category: 'GUI-World'
!
View methodsFor: 'initialization'
!
initialize
    subviews := Array new: 0.
    bounds := nil.
    ^ self
!
View methodsFor: 'accessing'
!
bounds
    ^ bounds
!
bounds: anObject
    bounds := anObject
!
subviews
    ^ subviews
!
subviews: anObject
    subviews := anObject
!
addSubview: aView
    | newArray index |
    newArray := Array new: subviews size + 1.
    index := 1.
    [ index <= subviews size ] whileTrue: [
        newArray at: index put: (subviews at: index).
        index := index + 1
    ].
    newArray at: index put: aView.
    subviews := newArray.
    ^ aView
!
contentBounds
    ^ bounds
!
View methodsFor: 'drawing'
!
drawOn: aDrawingTarget
    | canvas viewCanvas |
    canvas := aDrawingTarget asCanvas.
    viewCanvas := self drawingCanvasFrom: canvas.
    self drawContentsOn: viewCanvas.
    ^ self drawSubviewsOn: (self subviewCanvasFrom: viewCanvas) index: 1
!
drawingCanvasFrom: aCanvas
    bounds isNil ifTrue: [ ^ aCanvas ].
    ^ aCanvas clippedTo: bounds
!
subviewCanvasFrom: aCanvas
    ^ aCanvas
!
drawContentsOn: aCanvas
    ^ self
!
drawSubviewsOn: aCanvas index: index
    index > subviews size ifTrue: [ ^ self ].
    (subviews at: index) drawOn: aCanvas.
    ^ self drawSubviewsOn: aCanvas index: index + 1
!
SolidView methodsFor: 'initialization'
!
initialize
    super initialize.
    fillValue := 1.
    ^ self
!
SolidView methodsFor: 'accessing'
!
fillValue
    ^ fillValue
!
fillValue: anObject
    fillValue := anObject
!
SolidView methodsFor: 'drawing'
!
drawContentsOn: aCanvas
    bounds notNil ifTrue: [ aCanvas fillRectangle: bounds with: fillValue ].
    ^ self
!
LabelView methodsFor: 'initialization'
!
initialize
    super initialize.
    text := ''.
    font := StrikeFont new initializeDefault.
    color := 1.
    ^ self
!
LabelView methodsFor: 'accessing'
!
text
    ^ text
!
text: anObject
    text := anObject
!
font
    ^ font
!
font: anObject
    font := anObject
!
color
    ^ color
!
color: anObject
    color := anObject
!
LabelView methodsFor: 'drawing'
!
drawContentsOn: aCanvas
    bounds notNil ifTrue: [
        aCanvas drawString: text atX: ((bounds origin) x) y: ((bounds origin) y) font: font color: color
    ].
    ^ self
!
SystemWindow methodsFor: 'initialization'
!
initialize
    super initialize.
    fillValue := 0.
    borderValue := 1.
    titleBarValue := 1.
    titleBarHeight := 8.
    title := 'WINDOW'.
    titleColor := 0.
    titleFont := StrikeFont new initializeDefault.
    titleInset := 2.
    ^ self
!
SystemWindow methodsFor: 'accessing'
!
fillValue
    ^ fillValue
!
fillValue: anObject
    fillValue := anObject
!
borderValue
    ^ borderValue
!
borderValue: anObject
    borderValue := anObject
!
titleBarValue
    ^ titleBarValue
!
titleBarValue: anObject
    titleBarValue := anObject
!
titleBarHeight
    ^ titleBarHeight
!
titleBarHeight: anObject
    titleBarHeight := anObject
!
title
    ^ title
!
title: anObject
    title := anObject
!
titleColor
    ^ titleColor
!
titleColor: anObject
    titleColor := anObject
!
titleFont
    ^ titleFont
!
titleFont: anObject
    titleFont := anObject
!
titleInset
    ^ titleInset
!
titleInset: anObject
    titleInset := anObject
!
contentBounds
    | rect |
    bounds isNil ifTrue: [ ^ nil ].
    rect := Rectangle new.
    rect
        setX: ((bounds origin) x) + 1
        y: ((bounds origin) y) + titleBarHeight + 1
        width: bounds width - 2
        height: bounds height - titleBarHeight - 2.
    ^ rect
!
SystemWindow methodsFor: 'drawing'
!
drawContentsOn: aCanvas
    | x y w h |
    bounds isNil ifTrue: [ ^ self ].
    x := (bounds origin) x.
    y := (bounds origin) y.
    w := bounds width.
    h := bounds height.
    aCanvas frameRectangleX: x y: y width: w height: h with: borderValue.
    self drawInteriorOn: aCanvas atX: x y: y width: w height: h.
    ^ self
!
drawInteriorOn: aCanvas atX: x y: y width: w height: h
    | innerHeight |
    w <= 2 ifTrue: [ ^ self ].
    h <= 2 ifTrue: [ ^ self ].
    aCanvas fillRectangleX: x + 1 y: y + 1 width: w - 2 height: h - 2 with: fillValue.
    innerHeight := titleBarHeight.
    innerHeight > 0 ifTrue: [
        aCanvas fillRectangleX: x + 1 y: y + 1 width: w - 2 height: innerHeight with: titleBarValue.
        self drawTitleOn: aCanvas atX: x y: y
    ].
    ^ self
!
drawTitleOn: aCanvas atX: x y: y
    aCanvas drawString: title atX: x + titleInset y: y + 1 font: titleFont color: titleColor.
    ^ self
!
subviewCanvasFrom: aCanvas
    | rect |
    rect := self contentBounds.
    rect isNil ifTrue: [ ^ aCanvas ].
    ^ aCanvas clippedTo: rect
!
World methodsFor: 'initialization'
!
initializeWidth: aWidth height: aHeight depth: aDepth
    hostDisplay := Behavior hostDisplayOpenWidth: aWidth height: aHeight depth: aDepth.
    displayForm := Form new initializeWidth: aWidth height: aHeight depth: aDepth.
    rootView := View new initialize.
    inputSensor := InputSensor new.
    ^ self
!
World methodsFor: 'accessing'
!
hostDisplay
    ^ hostDisplay
!
displayForm
    ^ displayForm
!
rootView
    ^ rootView
!
inputSensor
    ^ inputSensor
!
addSubview: aView
    ^ rootView addSubview: aView
!
extent
    ^ displayForm extent
!
World methodsFor: 'drawing'
!
render
    displayForm clear.
    rootView drawOn: displayForm.
    hostDisplay presentForm: displayForm.
    ^ self
!
nextEvent
    ^ inputSensor nextEvent
!

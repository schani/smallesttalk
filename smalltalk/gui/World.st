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
View methodsFor: 'drawing'
!
drawOn: aForm
    ^ self drawSubviewsOn: aForm index: 1
!
drawSubviewsOn: aForm index: index
    index > subviews size ifTrue: [ ^ self ].
    (subviews at: index) drawOn: aForm.
    ^ self drawSubviewsOn: aForm index: index + 1
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
drawOn: aForm
    bounds notNil ifTrue: [
        aForm
            fillRectangleX: ((bounds origin) x)
            y: ((bounds origin) y)
            width: bounds width
            height: bounds height
            with: fillValue
    ].
    ^ super drawOn: aForm
!
LabelView methodsFor: 'initialization'
!
initialize
    super initialize.
    text := ''.
    font := BitmapFont new initializeDefault.
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
drawOn: aForm
    bounds notNil ifTrue: [
        aForm drawString: text atX: ((bounds origin) x) y: ((bounds origin) y) font: font color: color
    ].
    ^ super drawOn: aForm
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
    titleFont := BitmapFont new initializeDefault.
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
SystemWindow methodsFor: 'drawing'
!
drawOn: aForm
    | x y w h |
    bounds isNil ifTrue: [ ^ self ].
    x := (bounds origin) x.
    y := (bounds origin) y.
    w := bounds width.
    h := bounds height.
    aForm frameRectangleX: x y: y width: w height: h with: borderValue.
    self drawInteriorOn: aForm atX: x y: y width: w height: h.
    ^ super drawOn: aForm
!
drawInteriorOn: aForm atX: x y: y width: w height: h
    | innerHeight |
    w <= 2 ifTrue: [ ^ self ].
    h <= 2 ifTrue: [ ^ self ].
    aForm fillRectangleX: x + 1 y: y + 1 width: w - 2 height: h - 2 with: fillValue.
    innerHeight := titleBarHeight.
    innerHeight > 0 ifTrue: [
        aForm fillRectangleX: x + 1 y: y + 1 width: w - 2 height: innerHeight with: titleBarValue.
        self drawTitleOn: aForm atX: x y: y
    ].
    ^ self
!
drawTitleOn: aForm atX: x y: y
    aForm drawString: title atX: x + titleInset y: y + 1 font: titleFont color: titleColor.
    ^ self
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

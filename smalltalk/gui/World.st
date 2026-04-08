Behavior subclass: #View instanceVariableNames: 'bounds subviews' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
!
View subclass: #SolidView instanceVariableNames: 'fillValue' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
!
View subclass: #SystemWindow instanceVariableNames: 'fillValue borderValue titleBarValue titleBarHeight' classVariableNames: '' poolDictionaries: '' category: 'GUI-Views'
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
SystemWindow methodsFor: 'initialization'
!
initialize
    super initialize.
    fillValue := 0.
    borderValue := 1.
    titleBarValue := 1.
    titleBarHeight := 2.
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
SystemWindow methodsFor: 'drawing'
!
drawOn: aForm
    | x y w h innerHeight |
    bounds isNil ifTrue: [ ^ self ].
    x := (bounds origin) x.
    y := (bounds origin) y.
    w := bounds width.
    h := bounds height.
    aForm frameRectangleX: x y: y width: w height: h with: borderValue.
    (w > 2 and: [ h > 2 ]) ifTrue: [
        aForm fillRectangleX: x + 1 y: y + 1 width: w - 2 height: h - 2 with: fillValue
    ].
    innerHeight := titleBarHeight.
    (w > 2 and: [ h > 2 and: [ innerHeight > 0 ] ]) ifTrue: [
        aForm fillRectangleX: x + 1 y: y + 1 width: w - 2 height: innerHeight with: titleBarValue
    ].
    ^ super drawOn: aForm
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

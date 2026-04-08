Behavior subclass: #Point instanceVariableNames: 'x y' classVariableNames: '' poolDictionaries: '' category: 'GUI-Geometry'
!
Behavior subclass: #Rectangle instanceVariableNames: 'origin corner' classVariableNames: '' poolDictionaries: '' category: 'GUI-Geometry'
!
Behavior subclass: #Canvas instanceVariableNames: 'form clipRect origin' classVariableNames: '' poolDictionaries: '' category: 'GUI-Graphics'
!
Behavior subclass: #Form instanceVariableNames: 'width height depth bits' classVariableNames: '' poolDictionaries: '' category: 'GUI-Graphics'
!
Behavior subclass: #HostDisplay instanceVariableNames: 'handle width height depth' classVariableNames: '' poolDictionaries: '' category: 'GUI-Host'
!
Point methodsFor: 'accessing'
!
x
    ^ x
!
y
    ^ y
!
x: anObject
    x := anObject
!
y: anObject
    y := anObject
!
setX: anX y: aY
    x := anX.
    y := aY.
    ^ self
!
Point methodsFor: 'arithmetic'
!
+ aPoint
    | point |
    point := Point new.
    point setX: x + (aPoint x) y: y + (aPoint y).
    ^ point
!
- aPoint
    | point |
    point := Point new.
    point setX: x - (aPoint x) y: y - (aPoint y).
    ^ point
!
Rectangle methodsFor: 'accessing'
!
origin
    ^ origin
!
origin: anObject
    origin := anObject
!
corner
    ^ corner
!
corner: anObject
    corner := anObject
!
setOrigin: aPoint corner: anotherPoint
    origin := aPoint.
    corner := anotherPoint.
    ^ self
!
setX: xValue y: yValue width: widthValue height: heightValue
    origin := Point new setX: xValue y: yValue.
    corner := Point new setX: xValue + widthValue y: yValue + heightValue.
    ^ self
!
width
    ^ (corner x) - (origin x)
!
height
    ^ (corner y) - (origin y)
!
containsPoint: aPoint
    ^ ((aPoint x) >= (origin x))
        and: [ ((aPoint y) >= (origin y))
        and: [ ((aPoint x) < (corner x))
        and: [ (aPoint y) < (corner y) ] ] ]
!
translateBy: aPoint
    | rect |
    rect := Rectangle new.
    rect
        setX: (origin x) + (aPoint x)
        y: (origin y) + (aPoint y)
        width: self width
        height: self height.
    ^ rect
!
insetBy: amount
    | rect |
    rect := Rectangle new.
    rect
        setX: (origin x) + amount
        y: (origin y) + amount
        width: self width - (amount * 2)
        height: self height - (amount * 2).
    ^ rect
!
intersect: aRectangle
    | left top right bottom rect |
    aRectangle isNil ifTrue: [ ^ nil ].
    left := (origin x) > ((aRectangle origin) x)
        ifTrue: [ origin x ]
        ifFalse: [ (aRectangle origin) x ].
    top := (origin y) > ((aRectangle origin) y)
        ifTrue: [ origin y ]
        ifFalse: [ (aRectangle origin) y ].
    right := (corner x) < ((aRectangle corner) x)
        ifTrue: [ corner x ]
        ifFalse: [ (aRectangle corner) x ].
    bottom := (corner y) < ((aRectangle corner) y)
        ifTrue: [ corner y ]
        ifFalse: [ (aRectangle corner) y ].
    ((right <= left) or: [ bottom <= top ]) ifTrue: [ ^ nil ].
    rect := Rectangle new.
    rect setX: left y: top width: right - left height: bottom - top.
    ^ rect
!
Canvas methodsFor: 'initialization'
!
initializeOn: aForm clip: aRectangle origin: aPoint
    form := aForm.
    clipRect := aRectangle.
    origin := aPoint.
    ^ self
!
Canvas methodsFor: 'accessing'
!
form
    ^ form
!
clipRect
    ^ clipRect
!
origin
    ^ origin
!
asCanvas
    ^ self
!
clippedTo: aRectangle
    | rect canvas |
    rect := aRectangle translateBy: origin.
    canvas := Canvas new.
    canvas initializeOn: form clip: (clipRect intersect: rect) origin: origin.
    ^ canvas
!
translatedBy: aPoint
    | canvas |
    canvas := Canvas new.
    canvas initializeOn: form clip: clipRect origin: origin + aPoint.
    ^ canvas
!
withClip: aRectangle do: aBlock
    ^ aBlock value: (self clippedTo: aRectangle)
!
translatedBy: aPoint do: aBlock
    ^ aBlock value: (self translatedBy: aPoint)
!
Canvas methodsFor: 'drawing'
!
fillRectangleX: x y: y width: aWidth height: aHeight with: value
    | rect clipped |
    aWidth <= 0 ifTrue: [ ^ self ].
    aHeight <= 0 ifTrue: [ ^ self ].
    clipRect isNil ifTrue: [ ^ self ].
    rect := Rectangle new.
    rect setX: x + (origin x) y: y + (origin y) width: aWidth height: aHeight.
    clipped := rect intersect: clipRect.
    clipped isNil ifTrue: [ ^ self ].
    form
        fillRectangleX: ((clipped origin) x)
        y: ((clipped origin) y)
        width: clipped width
        height: clipped height
        with: value.
    ^ self
!
fillRectangle: aRectangle with: value
    ^ self
        fillRectangleX: ((aRectangle origin) x)
        y: ((aRectangle origin) y)
        width: aRectangle width
        height: aRectangle height
        with: value
!
frameRectangleX: x y: y width: aWidth height: aHeight with: value
    aWidth <= 0 ifTrue: [ ^ self ].
    aHeight <= 0 ifTrue: [ ^ self ].
    self fillRectangleX: x y: y width: aWidth height: 1 with: value.
    aHeight > 1 ifTrue: [ self fillRectangleX: x y: y + aHeight - 1 width: aWidth height: 1 with: value ].
    aHeight > 2 ifTrue: [
        self fillRectangleX: x y: y + 1 width: 1 height: aHeight - 2 with: value.
        aWidth > 1 ifTrue: [ self fillRectangleX: x + aWidth - 1 y: y + 1 width: 1 height: aHeight - 2 with: value ]
    ].
    ^ self
!
drawString: aString atX: x y: y font: aFont color: color
    clipRect isNil ifTrue: [ ^ self ].
    ^ aFont drawString: aString on: self atX: x y: y color: color
!
copyRectangleX: x y: y width: aWidth height: aHeight from: sourceForm atX: sourceX y: sourceY
    | rect clipped deltaX deltaY |
    aWidth <= 0 ifTrue: [ ^ self ].
    aHeight <= 0 ifTrue: [ ^ self ].
    clipRect isNil ifTrue: [ ^ self ].
    rect := Rectangle new.
    rect setX: x + (origin x) y: y + (origin y) width: aWidth height: aHeight.
    clipped := rect intersect: clipRect.
    clipped isNil ifTrue: [ ^ self ].
    deltaX := ((clipped origin) x) - (rect origin x).
    deltaY := ((clipped origin) y) - (rect origin y).
    form
        copyRectangleX: ((clipped origin) x)
        y: ((clipped origin) y)
        width: clipped width
        height: clipped height
        from: sourceForm
        atX: sourceX + deltaX
        y: sourceY + deltaY.
    ^ self
!
Form methodsFor: 'accessing'
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
depth
    ^ depth
!
depth: anObject
    depth := anObject
!
bits
    ^ bits
!
bits: anObject
    bits := anObject
!
initializeWidth: aWidth height: aHeight depth: aDepth
    width := aWidth.
    height := aHeight.
    depth := aDepth.
    bits := ByteArray new: self byteSize.
    ^ self
!
byteSize
    depth = 1 ifTrue: [ ^ ((width * height) + 7) bitShift: 0 - 3 ].
    depth = 8 ifTrue: [ ^ width * height ].
    ^ width * height
!
clear
    1 to: bits size do: [:index | bits at: index put: 0 ].
    ^ self
!
fillAllWith: value
    ^ self fillRectangleX: 0 y: 0 width: width height: height with: value
!
fillRectangle: aRectangle with: value
    ^ self
        fillRectangleX: ((aRectangle origin) x)
        y: ((aRectangle origin) y)
        width: aRectangle width
        height: aRectangle height
        with: value
!
frameRectangleX: x y: y width: aWidth height: aHeight with: value
    aWidth <= 0 ifTrue: [ ^ self ].
    aHeight <= 0 ifTrue: [ ^ self ].
    self fillRectangleX: x y: y width: aWidth height: 1 with: value.
    aHeight > 1 ifTrue: [ self fillRectangleX: x y: y + aHeight - 1 width: aWidth height: 1 with: value ].
    aHeight > 2 ifTrue: [
        self fillRectangleX: x y: y + 1 width: 1 height: aHeight - 2 with: value.
        aWidth > 1 ifTrue: [ self fillRectangleX: x + aWidth - 1 y: y + 1 width: 1 height: aHeight - 2 with: value ]
    ].
    ^ self
!
extent
    | array |
    array := Array new: 2.
    array at: 1 put: width.
    array at: 2 put: height.
    ^ array
!
fullBounds
    | rect |
    rect := Rectangle new.
    rect setX: 0 y: 0 width: width height: height.
    ^ rect
!
fullCanvas
    | canvas point |
    canvas := Canvas new.
    point := Point new setX: 0 y: 0.
    canvas initializeOn: self clip: self fullBounds origin: point.
    ^ canvas
!
asCanvas
    ^ self fullCanvas
!
HostDisplay methodsFor: 'accessing'
!
handle
    ^ handle
!
handle: anObject
    handle := anObject
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
depth
    ^ depth
!
depth: anObject
    depth := anObject
!
setHandle: aHandle width: aWidth height: aHeight depth: aDepth
    handle := aHandle.
    width := aWidth.
    height := aHeight.
    depth := aDepth.
    ^ self
!
extent
    | array |
    array := Array new: 2.
    array at: 1 put: width.
    array at: 2 put: height.
    ^ array
!

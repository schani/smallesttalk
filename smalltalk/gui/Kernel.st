Behavior subclass: #Point instanceVariableNames: 'x y' classVariableNames: '' poolDictionaries: '' category: 'GUI-Geometry'
!
Behavior subclass: #Rectangle instanceVariableNames: 'origin corner' classVariableNames: '' poolDictionaries: '' category: 'GUI-Geometry'
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

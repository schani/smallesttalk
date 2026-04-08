Behavior subclass: #BitmapFont instanceVariableNames: 'glyphWidth glyphHeight' classVariableNames: '' poolDictionaries: '' category: 'GUI-Fonts'
!
BitmapFont methodsFor: 'initialization'
!
initializeDefault
    glyphWidth := 5.
    glyphHeight := 7.
    ^ self
!
BitmapFont methodsFor: 'accessing'
!
glyphWidth
    ^ glyphWidth
!
glyphHeight
    ^ glyphHeight
!
advanceWidth
    ^ glyphWidth + 1
!
BitmapFont methodsFor: 'drawing'
!
drawString: aString on: aForm atX: x y: y color: color
    | index cursorX code |
    index := 1.
    cursorX := x.
    [ index <= aString size ] whileTrue: [
        code := aString at: index.
        self drawCode: code on: aForm atX: cursorX y: y color: color.
        cursorX := cursorX + self advanceWidth.
        index := index + 1
    ].
    ^ self
!
drawCode: code on: aForm atX: x y: y color: color
    | rows rowIndex rowBits col mask normalized |
    normalized := self normalizeCode: code.
    rows := self glyphRowsForCode: normalized.
    rowIndex := 1.
    [ rowIndex <= glyphHeight ] whileTrue: [
        rowBits := rows at: rowIndex.
        col := 0.
        [ col < glyphWidth ] whileTrue: [
            mask := 1 bitShift: (glyphWidth - 1) - col.
            ((rowBits bitAnd: mask) ~= 0) ifTrue: [
                aForm fillRectangleX: x + col y: y + rowIndex - 1 width: 1 height: 1 with: color
            ].
            col := col + 1
        ].
        rowIndex := rowIndex + 1
    ].
    ^ self
!
normalizeCode: code
    ((code >= 97) and: [ code <= 122 ]) ifTrue: [ ^ code - 32 ].
    ^ code
!
glyphRowsForCode: code
    code = 32 ifTrue: [ ^ #(0 0 0 0 0 0 0) ].
    code = 45 ifTrue: [ ^ #(0 0 0 31 0 0 0) ].
    code = 46 ifTrue: [ ^ #(0 0 0 0 0 12 12) ].
    code = 48 ifTrue: [ ^ #(14 17 19 21 25 17 14) ].
    code = 49 ifTrue: [ ^ #(4 12 4 4 4 4 14) ].
    code = 50 ifTrue: [ ^ #(14 17 1 2 4 8 31) ].
    code = 51 ifTrue: [ ^ #(30 1 1 14 1 1 30) ].
    code = 52 ifTrue: [ ^ #(2 6 10 18 31 2 2) ].
    code = 53 ifTrue: [ ^ #(31 16 16 30 1 1 30) ].
    code = 54 ifTrue: [ ^ #(14 16 16 30 17 17 14) ].
    code = 55 ifTrue: [ ^ #(31 1 2 4 8 8 8) ].
    code = 56 ifTrue: [ ^ #(14 17 17 14 17 17 14) ].
    code = 57 ifTrue: [ ^ #(14 17 17 15 1 1 14) ].
    code = 58 ifTrue: [ ^ #(0 12 12 0 12 12 0) ].
    code = 65 ifTrue: [ ^ #(14 17 17 31 17 17 17) ].
    code = 66 ifTrue: [ ^ #(30 17 17 30 17 17 30) ].
    code = 67 ifTrue: [ ^ #(14 17 16 16 16 17 14) ].
    code = 68 ifTrue: [ ^ #(30 17 17 17 17 17 30) ].
    code = 69 ifTrue: [ ^ #(31 16 16 30 16 16 31) ].
    code = 70 ifTrue: [ ^ #(31 16 16 30 16 16 16) ].
    code = 71 ifTrue: [ ^ #(14 17 16 23 17 17 14) ].
    code = 72 ifTrue: [ ^ #(17 17 17 31 17 17 17) ].
    code = 73 ifTrue: [ ^ #(14 4 4 4 4 4 14) ].
    code = 74 ifTrue: [ ^ #(1 1 1 1 17 17 14) ].
    code = 75 ifTrue: [ ^ #(17 18 20 24 20 18 17) ].
    code = 76 ifTrue: [ ^ #(16 16 16 16 16 16 31) ].
    code = 77 ifTrue: [ ^ #(17 27 21 21 17 17 17) ].
    code = 78 ifTrue: [ ^ #(17 17 25 21 19 17 17) ].
    code = 79 ifTrue: [ ^ #(14 17 17 17 17 17 14) ].
    code = 80 ifTrue: [ ^ #(30 17 17 30 16 16 16) ].
    code = 81 ifTrue: [ ^ #(14 17 17 17 21 18 13) ].
    code = 82 ifTrue: [ ^ #(30 17 17 30 20 18 17) ].
    code = 83 ifTrue: [ ^ #(15 16 16 14 1 1 30) ].
    code = 84 ifTrue: [ ^ #(31 4 4 4 4 4 4) ].
    code = 85 ifTrue: [ ^ #(17 17 17 17 17 17 14) ].
    code = 86 ifTrue: [ ^ #(17 17 17 17 17 10 4) ].
    code = 87 ifTrue: [ ^ #(17 17 17 21 21 21 10) ].
    code = 88 ifTrue: [ ^ #(17 17 10 4 10 17 17) ].
    code = 89 ifTrue: [ ^ #(17 17 10 4 4 4 4) ].
    code = 90 ifTrue: [ ^ #(31 1 2 4 8 16 31) ].
    ^ #(31 17 2 4 8 0 8)
!
Form methodsFor: 'text'
!
drawString: aString atX: x y: y font: aFont color: color
    ^ (self asCanvas) drawString: aString atX: x y: y font: aFont color: color
!

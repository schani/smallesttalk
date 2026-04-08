Behavior subclass: #BitmapFont instanceVariableNames: 'glyphWidth glyphHeight' classVariableNames: '' poolDictionaries: '' category: 'GUI-Fonts'
!
BitmapFont subclass: #StrikeFont instanceVariableNames: '' classVariableNames: '' poolDictionaries: '' category: 'GUI-Fonts'
!
BitmapFont methodsFor: 'initialization'
!
initializeDefault
    glyphWidth := 5.
    glyphHeight := 7.
    ^ self
!
StrikeFont methodsFor: 'initialization'
!
initializeDefault
    ^ super initializeDefault
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
    ^ code
!
glyphRowsForCode: code
    code = 32 ifTrue: [ ^ #(0 0 0 0 0 0 0) ].
    code = 35 ifTrue: [ ^ #(10 31 10 10 31 10 0) ].
    code = 39 ifTrue: [ ^ #(4 4 8 0 0 0 0) ].
    code = 40 ifTrue: [ ^ #(2 4 8 8 8 4 2) ].
    code = 41 ifTrue: [ ^ #(8 4 2 2 2 4 8) ].
    code = 42 ifTrue: [ ^ #(0 10 4 31 4 10 0) ].
    code = 43 ifTrue: [ ^ #(0 4 4 31 4 4 0) ].
    code = 44 ifTrue: [ ^ #(0 0 0 0 12 12 8) ].
    code = 45 ifTrue: [ ^ #(0 0 0 31 0 0 0) ].
    code = 46 ifTrue: [ ^ #(0 0 0 0 0 12 12) ].
    code = 47 ifTrue: [ ^ #(1 2 4 8 16 0 0) ].
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
    code = 59 ifTrue: [ ^ #(0 12 12 0 12 12 8) ].
    code = 60 ifTrue: [ ^ #(2 4 8 16 8 4 2) ].
    code = 61 ifTrue: [ ^ #(0 31 0 31 0 0 0) ].
    code = 62 ifTrue: [ ^ #(8 4 2 1 2 4 8) ].
    code = 63 ifTrue: [ ^ #(14 17 1 2 4 0 4) ].
    code = 64 ifTrue: [ ^ #(14 17 1 13 21 21 14) ].
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
    code = 91 ifTrue: [ ^ #(14 8 8 8 8 8 14) ].
    code = 93 ifTrue: [ ^ #(14 2 2 2 2 2 14) ].
    code = 94 ifTrue: [ ^ #(4 10 17 0 0 0 0) ].
    code = 95 ifTrue: [ ^ #(0 0 0 0 0 0 31) ].
    code = 96 ifTrue: [ ^ #(8 4 2 0 0 0 0) ].
    code = 97 ifTrue: [ ^ #(0 0 14 1 15 17 15) ].
    code = 98 ifTrue: [ ^ #(16 16 22 25 17 17 30) ].
    code = 99 ifTrue: [ ^ #(0 0 14 16 16 17 14) ].
    code = 100 ifTrue: [ ^ #(1 1 13 19 17 17 15) ].
    code = 101 ifTrue: [ ^ #(0 0 14 17 31 16 14) ].
    code = 102 ifTrue: [ ^ #(6 8 8 30 8 8 8) ].
    code = 103 ifTrue: [ ^ #(0 15 17 17 15 1 14) ].
    code = 104 ifTrue: [ ^ #(16 16 22 25 17 17 17) ].
    code = 105 ifTrue: [ ^ #(4 0 12 4 4 4 14) ].
    code = 106 ifTrue: [ ^ #(2 0 6 2 2 18 12) ].
    code = 107 ifTrue: [ ^ #(16 16 18 20 24 20 18) ].
    code = 108 ifTrue: [ ^ #(12 4 4 4 4 4 14) ].
    code = 109 ifTrue: [ ^ #(0 0 26 21 21 21 21) ].
    code = 110 ifTrue: [ ^ #(0 0 22 25 17 17 17) ].
    code = 111 ifTrue: [ ^ #(0 0 14 17 17 17 14) ].
    code = 112 ifTrue: [ ^ #(0 0 30 17 30 16 16) ].
    code = 113 ifTrue: [ ^ #(0 0 13 19 15 1 1) ].
    code = 114 ifTrue: [ ^ #(0 0 22 25 16 16 16) ].
    code = 115 ifTrue: [ ^ #(0 0 15 16 14 1 30) ].
    code = 116 ifTrue: [ ^ #(8 8 30 8 8 8 6) ].
    code = 117 ifTrue: [ ^ #(0 0 17 17 17 19 13) ].
    code = 118 ifTrue: [ ^ #(0 0 17 17 17 10 4) ].
    code = 119 ifTrue: [ ^ #(0 0 17 17 21 21 10) ].
    code = 120 ifTrue: [ ^ #(0 0 17 10 4 10 17) ].
    code = 121 ifTrue: [ ^ #(0 0 17 17 15 1 14) ].
    code = 122 ifTrue: [ ^ #(0 0 31 2 4 8 31) ].
    code = 123 ifTrue: [ ^ #(2 4 4 8 4 4 2) ].
    code = 124 ifTrue: [ ^ #(4 4 4 4 4 4 4) ].
    code = 125 ifTrue: [ ^ #(8 4 4 2 4 4 8) ].
    ^ #(31 17 2 4 8 0 8)
!
Form methodsFor: 'text'
!
drawString: aString atX: x y: y font: aFont color: color
    ^ (self asCanvas) drawString: aString atX: x y: y font: aFont color: color
!

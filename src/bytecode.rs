pub const PUSH_INST_VAR_BASE: u8 = 0x00;
pub const PUSH_TEMP_BASE: u8 = 0x10;
pub const PUSH_LITERAL_BASE: u8 = 0x20;
pub const PUSH_LIT_VAR_BASE: u8 = 0x30;
pub const POP_STORE_INST_VAR_BASE: u8 = 0x40;
pub const POP_STORE_TEMP_BASE: u8 = 0x48;

pub const PUSH_SELF: u8 = 0x50;
pub const PUSH_NIL: u8 = 0x51;
pub const PUSH_TRUE: u8 = 0x52;
pub const PUSH_FALSE: u8 = 0x53;
pub const PUSH_MINUS_ONE: u8 = 0x54;
pub const PUSH_ZERO: u8 = 0x55;
pub const PUSH_ONE: u8 = 0x56;
pub const PUSH_TWO: u8 = 0x57;
pub const DUP: u8 = 0x58;
pub const POP: u8 = 0x59;

pub const SEND_SHORT_BASE: u8 = 0x60;
pub const SEND_SPECIAL_BASE: u8 = 0x70;

pub const PUSH_INST_VAR_EXT: u8 = 0x80;
pub const PUSH_TEMP_EXT: u8 = 0x81;
pub const PUSH_LITERAL_EXT: u8 = 0x82;
pub const PUSH_LIT_VAR_EXT: u8 = 0x83;
pub const POP_STORE_INST_VAR_EXT: u8 = 0x84;
pub const POP_STORE_TEMP_EXT: u8 = 0x85;
pub const SEND_EXT: u8 = 0x86;
pub const SUPER_SEND_EXT: u8 = 0x87;
pub const JUMP_FORWARD: u8 = 0x88;
pub const JUMP_BACK: u8 = 0x89;
pub const JUMP_TRUE: u8 = 0x8A;
pub const JUMP_FALSE: u8 = 0x8B;
pub const PUSH_NEW_ARRAY: u8 = 0x8C;
pub const PUSH_SMALL_INT_EXT: u8 = 0x8D;

pub const EXTENDED_SEND: u8 = 0xC0;
pub const EXTENDED_SUPER_SEND: u8 = 0xC1;
pub const JUMP_FORWARD_LONG: u8 = 0xC2;
pub const JUMP_BACK_LONG: u8 = 0xC3;
pub const JUMP_TRUE_LONG: u8 = 0xC4;
pub const JUMP_FALSE_LONG: u8 = 0xC5;

pub const PUSH_CLOSURE: u8 = 0xE0;
pub const RETURN_TOP: u8 = 0xE1;
pub const RETURN_SELF: u8 = 0xE2;
pub const RETURN_NIL: u8 = 0xE3;
pub const BLOCK_RETURN: u8 = 0xE4;

pub const SPECIAL_SEND_SELECTORS: [&str; 16] = [
    "+",
    "-",
    "*",
    "/",
    "<",
    ">",
    "<=",
    ">=",
    "=",
    "~=",
    "bitAnd:",
    "bitOr:",
    "bitShift:",
    "@",
    "at:",
    "at:put:",
];

pub fn selector_arity(name: &str) -> usize {
    let keyword_count = name.bytes().filter(|byte| *byte == b':').count();
    if keyword_count > 0 {
        keyword_count
    } else if name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        0
    } else {
        1
    }
}

pub fn disassemble(bytes: &[u8]) -> Vec<String> {
    let mut ip = 0usize;
    let mut out = Vec::new();
    while ip < bytes.len() {
        let start = ip;
        let op = bytes[ip];
        ip += 1;
        let text = match op {
            0x00..=0x0f => format!("pushInstVar {}", op - PUSH_INST_VAR_BASE),
            0x10..=0x1f => format!("pushTemp {}", op - PUSH_TEMP_BASE),
            0x20..=0x2f => format!("pushLiteral {}", op - PUSH_LITERAL_BASE),
            0x30..=0x3f => format!("pushLitVar {}", op - PUSH_LIT_VAR_BASE),
            0x40..=0x47 => format!("popStoreInstVar {}", op - POP_STORE_INST_VAR_BASE),
            0x48..=0x4f => format!("popStoreTemp {}", op - POP_STORE_TEMP_BASE),
            PUSH_SELF => "pushSelf".to_string(),
            PUSH_NIL => "pushNil".to_string(),
            PUSH_TRUE => "pushTrue".to_string(),
            PUSH_FALSE => "pushFalse".to_string(),
            PUSH_MINUS_ONE => "push -1".to_string(),
            PUSH_ZERO => "push 0".to_string(),
            PUSH_ONE => "push 1".to_string(),
            PUSH_TWO => "push 2".to_string(),
            DUP => "dup".to_string(),
            POP => "pop".to_string(),
            0x60..=0x6f => format!("sendShort lit={}", op - SEND_SHORT_BASE),
            0x70..=0x7f => format!(
                "sendSpecial {}",
                SPECIAL_SEND_SELECTORS[(op - 0x70) as usize]
            ),
            PUSH_INST_VAR_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("pushInstVar {arg}")
            }
            PUSH_TEMP_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("pushTemp {arg}")
            }
            PUSH_LITERAL_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("pushLiteral {arg}")
            }
            PUSH_LIT_VAR_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("pushLitVar {arg}")
            }
            POP_STORE_INST_VAR_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("popStoreInstVar {arg}")
            }
            POP_STORE_TEMP_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("popStoreTemp {arg}")
            }
            SEND_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("send {arg}")
            }
            SUPER_SEND_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("superSend {arg}")
            }
            JUMP_FORWARD => {
                let arg = bytes[ip];
                ip += 1;
                format!("jumpForward {arg}")
            }
            JUMP_BACK => {
                let arg = bytes[ip];
                ip += 1;
                format!("jumpBack {arg}")
            }
            JUMP_TRUE => {
                let arg = bytes[ip];
                ip += 1;
                format!("jumpTrue {arg}")
            }
            JUMP_FALSE => {
                let arg = bytes[ip];
                ip += 1;
                format!("jumpFalse {arg}")
            }
            PUSH_NEW_ARRAY => {
                let arg = bytes[ip];
                ip += 1;
                format!("pushNewArray {arg}")
            }
            PUSH_SMALL_INT_EXT => {
                let arg = bytes[ip];
                ip += 1;
                format!("push SmallInt {arg}")
            }
            EXTENDED_SEND => {
                let hi = bytes[ip];
                let lo = bytes[ip + 1];
                ip += 2;
                format!("extendedSend lit={hi} argc={lo}")
            }
            EXTENDED_SUPER_SEND => {
                let hi = bytes[ip];
                let lo = bytes[ip + 1];
                ip += 2;
                format!("extendedSuperSend lit={hi} argc={lo}")
            }
            JUMP_FORWARD_LONG => {
                let hi = bytes[ip];
                let lo = bytes[ip + 1];
                ip += 2;
                format!("jumpForwardLong {}", ((hi as u16) << 8) | lo as u16)
            }
            JUMP_BACK_LONG => {
                let hi = bytes[ip];
                let lo = bytes[ip + 1];
                ip += 2;
                format!("jumpBackLong {}", ((hi as u16) << 8) | lo as u16)
            }
            JUMP_TRUE_LONG => {
                let hi = bytes[ip];
                let lo = bytes[ip + 1];
                ip += 2;
                format!("jumpTrueLong {}", ((hi as u16) << 8) | lo as u16)
            }
            JUMP_FALSE_LONG => {
                let hi = bytes[ip];
                let lo = bytes[ip + 1];
                ip += 2;
                format!("jumpFalseLong {}", ((hi as u16) << 8) | lo as u16)
            }
            PUSH_CLOSURE => {
                let args = bytes[ip];
                let copied = bytes[ip + 1];
                let size = ((bytes[ip + 2] as u16) << 8) | bytes[ip + 3] as u16;
                ip += 4;
                format!("pushClosure args={args} copied={copied} size={size}")
            }
            RETURN_TOP => "returnTop".to_string(),
            RETURN_SELF => "returnSelf".to_string(),
            RETURN_NIL => "returnNil".to_string(),
            BLOCK_RETURN => "blockReturn".to_string(),
            other => format!("unknown 0x{other:02x}"),
        };
        out.push(format!("{start:04x}: {text}"));
    }
    out
}

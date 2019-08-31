use std::num::ParseIntError;
use std::iter;

#[derive(Debug,Clone)]
pub enum ParseError {
    UnknownInstruction(String),
    UnknownRegister(String),
    MissingOperand(String, usize),
    BadInt(ParseIntError),
    Empty,
}

pub trait Parser: Sized {
    fn parse(s: &str) -> Result<Self, ParseError>;
}

#[derive(Debug,Clone,Copy,PartialEq,PartialOrd)]
pub struct Heading(pub f32);  // nominally radians

impl Heading {
    const DIVISOR: usize = 256usize;

    pub fn to_integral(&self) -> isize {
        ((self.0 / (2.0 * ::std::f32::consts::PI)) * (Heading::DIVISOR as f32)) as isize
    }
}

impl From<isize> for Heading {
    fn from(v: isize) -> Heading {
        Heading(2.0 * ::std::f32::consts::PI * ((v / (Heading::DIVISOR as isize)) as f32))
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Register {
    A,
    B,
    X,
    T,
}

impl Parser for Register {
    fn parse(s: &str) -> Result<Register, ParseError> {
        match s.trim() {
            "a" | "A" => Ok(Register::A),
            "b" | "B" => Ok(Register::B),
            "x" | "X" => Ok(Register::X),
            "t" | "T" => Ok(Register::T),
            v => Err(ParseError::UnknownRegister(v.into())),
        }
    }
}

#[derive(Debug,Clone,Default)]
pub struct Registers {
    pub a: isize,
    pub b: isize,
    pub t: bool,
    pub x: usize,
}

#[derive(Debug,Clone)]
pub enum Target {
    Reg(Register),
    Mem(usize),
}

impl Parser for Target {
    fn parse(s: &str) -> Result<Target, ParseError> {
        if s == "*" {
            return Ok(Target::Reg(Register::X));
        }
        if s.starts_with("(") {
            return Ok(Target::Mem(usize::from_str_radix(&s[1..], 10).map_err(ParseError::BadInt)?));
        }
        Ok(Target::Reg(Register::parse(s)?))
    }
}


#[derive(Debug,Clone)]
pub enum Valuant {
    Target(Target),
    Const(isize),
}

impl Parser for Valuant {
    fn parse(s: &str) -> Result<Valuant, ParseError> {
        match isize::from_str_radix(s, 10) {
            Ok(i) => Ok(Valuant::Const(i)),
            Err(_) => Ok(Valuant::Target(Target::parse(s)?)),
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl BinOp {
    pub fn apply(&self, left: isize, right: isize) -> Option<isize> {
        match self {
            &BinOp::Add => Some(left + right),
            &BinOp::Sub => Some(left - right),
            &BinOp::Mul => Some(left * right),
            &BinOp::Div => if right == 0 { None } else { Some(left / right) },
            &BinOp::Mod => if right == 0 { None } else { Some(left % right) },
        }
    }
}

impl Parser for BinOp {
    fn parse(s: &str) -> Result<BinOp, ParseError> {
        match s {
            "add" => Ok(BinOp::Add),
            "sub" => Ok(BinOp::Sub),
            "mul" => Ok(BinOp::Mul),
            "div" => Ok(BinOp::Div),
            "mod" => Ok(BinOp::Mod),
            _ => Err(ParseError::UnknownInstruction(s.into())),
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq)]
pub enum UpCall {
    None,
    Scan,
    Fire,
    Aim(Heading),
    Turn(Heading),
    Temp,
    GPS,
    Move,
    Explode,
    Yield,
}

#[derive(Debug,Clone)]
pub enum UpCallInstr {
    None,
    Scan,
    Fire,
    Aim(Valuant),
    Turn(Valuant),
    GPS,
    Temp,
    Move,
    Explode,
    Yield,
}

impl UpCallInstr {
    pub fn to_upcall(&self) -> Option<UpCall> {
        match self {
            &UpCallInstr::None => Some(UpCall::None),
            &UpCallInstr::Scan => Some(UpCall::Scan),
            &UpCallInstr::Fire => Some(UpCall::Fire),
            &UpCallInstr::GPS => Some(UpCall::GPS),
            &UpCallInstr::Temp => Some(UpCall::Temp),
            &UpCallInstr::Move => Some(UpCall::Move),
            &UpCallInstr::Explode => Some(UpCall::Explode),
            &UpCallInstr::Yield => Some(UpCall::Yield),
            _ => None,
        }
    }
}

impl Parser for UpCallInstr {
    fn parse(s: &str) -> Result<UpCallInstr, ParseError> {
        match s {
            "none" => Ok(UpCallInstr::None),
            "scan" => Ok(UpCallInstr::Scan),
            "fire" => Ok(UpCallInstr::Fire),
            "gps" => Ok(UpCallInstr::GPS),
            "temp" => Ok(UpCallInstr::Temp),
            "move" => Ok(UpCallInstr::Move),
            "explode" => Ok(UpCallInstr::Explode),
            _ => Err(ParseError::UnknownInstruction(s.into())),
        }
    }
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Comparison {
    Less,
    Equal,
}

impl Comparison {
    pub fn compare(&self, left: isize, right: isize) -> bool {
        match self {
            &Comparison::Less => left < right,
            &Comparison::Equal => left == right,
        }
    }
}

impl Parser for Comparison {
    fn parse(s: &str) -> Result<Comparison, ParseError> {
        match s {
            "tlt" => Ok(Comparison::Less),
            "teq" => Ok(Comparison::Equal),
            _ => Err(ParseError::UnknownInstruction(s.into())),
        }
    }
}

#[derive(Debug,Clone)]
pub enum Instruction {
    BinOp {
        binop: BinOp,
        left: Valuant,
        right: Valuant,
        dest: Target,
    },
    Compare {
        comparison: Comparison,
        left: Valuant,
        right: Valuant,
    },
    UpCall(UpCallInstr),
    Jump(isize),
    JumpIfT(isize),
}

pub fn get_using<T, F: Fn(&str) -> Result<T, ParseError>>(words: &Vec<&str>, idx: usize, f: &F) -> Result<T, ParseError> {
    words.get(idx).map(|&x| x).ok_or_else(|| ParseError::MissingOperand((*words.first().unwrap()).into(), idx)).and_then(f)
}

impl Parser for Instruction {
    fn parse(s: &str) -> Result<Instruction, ParseError> {
        let words: Vec<&str> = s.split_whitespace().collect();
        match words.first() {
            Some(word) => match &*word.to_lowercase() {
                "load" => Ok(Instruction::BinOp {
                    binop: BinOp::Add,
                    left: Valuant::Const(0isize),
                    right: get_using(&words, 1, &Valuant::parse)?,
                    dest: get_using(&words, 2, &Target::parse)?,
                }),
                "write" => {
                    let res = Ok(Instruction::BinOp {
                        binop: BinOp::Add,
                        left: Valuant::Const(0isize),
                        right: get_using(&words, 1, &Valuant::parse)?,
                        dest: get_using(&words, 2, &Target::parse)?,
                    });
                    if let Ok(Instruction::BinOp{dest: Target::Reg(_), ..}) = res {
                        println!("warn: write to non-memory will be treated as normal load: {:?}", s);
                    }
                    res
                },
                op @ "add" | op @ "sub" | op @ "mul" | op @ "div" | op @ "mod" => Ok(Instruction::BinOp {
                    binop: BinOp::parse(op)?,
                    left: get_using(&words, 1, &Valuant::parse)?,
                    right: get_using(&words, 2, &Valuant::parse)?,
                    dest: get_using(&words, 3, &Target::parse)?,
                }),
                op @ "tlt" | op @ "teq" => Ok(Instruction::Compare {
                    comparison: Comparison::parse(op)?,
                    left: get_using(&words, 1, &Valuant::parse)?,
                    right: get_using(&words, 2, &Valuant::parse)?,
                }),
                op @ "scan" | op @ "fire" | op @ "gps" | op @ "move" | op @ "temp" => Ok(Instruction::UpCall(UpCallInstr::parse(op)?)),
                "aim" => Ok(Instruction::UpCall(UpCallInstr::Aim(get_using(&words, 1, &Valuant::parse)?))),
                "turn" => Ok(Instruction::UpCall(UpCallInstr::Turn(get_using(&words, 1, &Valuant::parse)?))),
                "nop" => Ok(Instruction::UpCall(UpCallInstr::Yield)),
                "jmp" => Ok(Instruction::Jump(
                    words.get(1).ok_or_else(|| ParseError::MissingOperand((*word as &str).into(), 1)).and_then(|o| {
                        isize::from_str_radix(o, 10).map_err(|e| ParseError::BadInt(e))
                    })?
                )),
                "jmpif" => Ok(Instruction::JumpIfT(
                    words.get(1).ok_or_else(|| ParseError::MissingOperand((*word as &str).into(), 1)).and_then(|o| {
                        isize::from_str_radix(o, 10).map_err(|e| ParseError::BadInt(e))
                    })?
                )),
                w => Err(ParseError::UnknownInstruction(w.into())),
            },
            None => Err(ParseError::Empty),
        }
    }
}

#[derive(Debug,Clone)]
pub struct State {
    pub mem: Vec<isize>,
    pub regs: Registers,
}

impl State {
    pub fn evaluate(&self, v: &Valuant) -> isize {
        match v {
            &Valuant::Target(Target::Reg(reg)) => match reg {
                Register::A => self.regs.a,
                Register::B => self.regs.b,
                Register::X => self.evaluate(&Valuant::Target(Target::Mem(self.regs.t as usize))),
                Register::T => if self.regs.t { 1 } else { 0 },
            },
            &Valuant::Target(Target::Mem(addr)) => self.mem.get(addr).map(|&x| x).unwrap_or_else(Default::default),
            &Valuant::Const(c) => c,
        }
    }

    pub fn load(&mut self, t: &Target, v: isize) {
        match t {
            &Target::Reg(ref r) => match r {
                &Register::A => self.regs.a = v,
                &Register::B => self.regs.b = v,
                &Register::X => self.regs.x = v as usize,
                &Register::T => self.regs.t = v != 0,
            },
            &Target::Mem(addr) => { self.mem.get_mut(addr).map(|x| *x = v); },
        }
    }

    pub fn exec(&mut self, inst: &Instruction) -> (UpCall, Option<isize>) {
        match inst {
            &Instruction::BinOp { ref binop, ref left, ref right, ref dest } => {
                let lval = self.evaluate(left);
                let rval = self.evaluate(right);
                match binop.apply(lval, rval) {
                    None => (UpCall::Explode, None),
                    Some(v) => {
                        self.load(dest, v);
                        (UpCall::None, None)
                    },
                }
            },
            &Instruction::Compare { ref comparison, ref left, ref right } => {
                let lval = self.evaluate(left);
                let rval = self.evaluate(right);
                self.regs.t = comparison.compare(lval, rval);
                (UpCall::None, None)
            },
            &Instruction::UpCall(ref uci) => match uci.to_upcall() {
                Some(uc) => (uc, None),
                None => match uci {
                    &UpCallInstr::Aim(ref v) => (UpCall::Aim(self.evaluate(v).into()), None),
                    &UpCallInstr::Turn(ref v) => (UpCall::Turn(self.evaluate(v).into()), None),
                    _ => unreachable!(),
                },
            },
            &Instruction::Jump(d) => (UpCall::None, Some(d)),
            &Instruction::JumpIfT(d) => (UpCall::None, if self.regs.t { Some(d) } else { None }),
        }
    }
}

#[derive(Debug,Clone)]
pub struct StateBuilder {
    pub mem_size: usize,
    pub regs: Registers,
}

impl Default for StateBuilder {
    fn default() -> StateBuilder {
        StateBuilder {
            mem_size: 256usize,
            regs: Default::default(),
        }
    }
}

impl StateBuilder {
    pub fn with_mem_size(self, sz: usize) -> StateBuilder {
        StateBuilder { mem_size: sz, ..self }
    }

    pub fn with_regs(self, regs: Registers) -> StateBuilder {
        StateBuilder { regs: regs, ..self }
    }

    pub fn build(self) -> State {
        State {
            mem: iter::repeat(0isize).take(self.mem_size).collect(),
            regs: self.regs,
        }
    }
}

#[derive(Debug,Clone)]
pub struct Program {
    pub instrs: Vec<Instruction>,
}

impl Parser for Program {
    fn parse(src: &str) -> Result<Program, ParseError> {
        let mut instrs: Vec<Instruction> = Vec::new();
        for line in src.lines() {
            match Instruction::parse(line) {
                Ok(i) => instrs.push(i),
                Err(ParseError::Empty) => (),
                Err(e) => return Err(e),
            }
        }
        Ok(Program{
            instrs: instrs,
        })
    }
}

#[derive(Debug,Clone)]
pub struct VMConfiguration {
    pub steps: usize,
}

impl Default for VMConfiguration {
    fn default() -> VMConfiguration {
        VMConfiguration {
            steps: 30,
        }
    }
}

#[derive(Debug,Clone)]
pub struct VM {
    pub config: VMConfiguration,
    pub state: State,
    pub prog: Program,
    pub pc: usize,
}

impl VM {
    pub fn new(p: Program, s: State) -> VM {
        VM {
            config: Default::default(),
            state: s,
            prog: p,
            pc: 0usize,
        }
    }

    pub fn run(&mut self) -> UpCall {
        for _step in 0..self.config.steps {
            if self.pc >= self.prog.instrs.len() {
                self.pc = 0
            }
            let (upcall, offset) = self.state.exec(&self.prog.instrs[self.pc]);
            let (mut pc, _) = self.pc.overflowing_add(1usize);
            if let Some(i) = offset {
                if i >= 0 {
                    pc = self.pc.saturating_add(i as usize);
                } else {
                    pc = self.pc.saturating_sub((-i) as usize);
                }
            }
            self.pc = pc;
            match upcall {
                UpCall::None => continue,
                v => return v,
            }
        }
        UpCall::None
    }
}

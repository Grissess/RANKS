use std::sync::{Arc, Mutex};

use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use wasmi::{
    nan_preserving_float::{F32, F64},
    ExternVal, Externals, FuncInstance, FuncInvocation, FuncRef, HostError, ImportsBuilder,
    ModuleImportResolver, ModuleInstance, ResumableError, RuntimeArgs, RuntimeValue, Signature,
    Trap, TrapKind, ValueType,
};

#[derive(Clone, Copy, Debug)]
enum HostCall {
    Upcall(UpcallId),
    UnaryOpF32(UnaryOp),
    BinaryOpF32(BinaryOp),
    UnaryOpF64(UnaryOp),
    BinaryOpF64(BinaryOp),
}

const B0: usize = 0;
const B1: usize = NUM_UPCALLS;
const B2: usize = NUM_UPCALLS + NUM_UNOPS;
const B3: usize = NUM_UPCALLS + NUM_UNOPS + NUM_BINOPS;
const B4: usize = NUM_UPCALLS + 2 * NUM_UNOPS + NUM_BINOPS;
const B5: usize = NUM_UPCALLS + 2 * NUM_UNOPS + 2 * NUM_BINOPS;

impl HostCall {
    pub fn from_name(name: &str) -> Result<Self, ()> {
        match name {
            "scan" => Ok(HostCall::Upcall(UpcallId::Scan)),
            "fire" => Ok(HostCall::Upcall(UpcallId::Fire)),
            "aim" => Ok(HostCall::Upcall(UpcallId::Aim)),
            "turn" => Ok(HostCall::Upcall(UpcallId::Turn)),
            "gpsx" => Ok(HostCall::Upcall(UpcallId::GPSX)),
            "gpsy" => Ok(HostCall::Upcall(UpcallId::GPSY)),
            "temp" => Ok(HostCall::Upcall(UpcallId::Temp)),
            "forward" => Ok(HostCall::Upcall(UpcallId::Forward)),
            "explode" => Ok(HostCall::Upcall(UpcallId::Explode)),
            "yield" => Ok(HostCall::Upcall(UpcallId::Yield)),
            "abs_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Abs)),
            "acos_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Acos)),
            "acosh_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Acosh)),
            "asin_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Asin)),
            "asinh_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Asinh)),
            "atan_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Atan)),
            "atanh_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Atanh)),
            "cbrt_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Cbrt)),
            "ceil_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Ceil)),
            "cos_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Cos)),
            "cosh_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Cosh)),
            "exp_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Exp)),
            "exp2_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Exp2)),
            "expm1_float" => Ok(HostCall::UnaryOpF32(UnaryOp::ExpM1)),
            "floor_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Floor)),
            "fract_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Fract)),
            "ln_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Ln)),
            "ln1p_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Ln1p)),
            "log10_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Log10)),
            "log2_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Log2)),
            "recip_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Recip)),
            "round_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Round)),
            "signum_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Signum)),
            "sin_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Sin)),
            "sinh_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Sinh)),
            "sqrt_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Sqrt)),
            "tan_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Tan)),
            "tanh_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Tanh)),
            "trunc_float" => Ok(HostCall::UnaryOpF32(UnaryOp::Trunc)),
            "atan2_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Atan2)),
            "copysign_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Copysign)),
            "div_euclid_float" => Ok(HostCall::BinaryOpF32(BinaryOp::DivEuclid)),
            "hypot_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Hypot)),
            "log_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Log)),
            "max_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Max)),
            "min_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Min)),
            "powf_float" => Ok(HostCall::BinaryOpF32(BinaryOp::Powf)),
            "rem_euclid_float" => Ok(HostCall::BinaryOpF32(BinaryOp::RemEuclid)),
            "abs_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Abs)),
            "acos_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Acos)),
            "acosh_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Acosh)),
            "asin_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Asin)),
            "asinh_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Asinh)),
            "atan_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Atan)),
            "atanh_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Atanh)),
            "cbrt_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Cbrt)),
            "ceil_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Ceil)),
            "cos_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Cos)),
            "cosh_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Cosh)),
            "exp_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Exp)),
            "exp2_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Exp2)),
            "expm1_double" => Ok(HostCall::UnaryOpF64(UnaryOp::ExpM1)),
            "floor_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Floor)),
            "fract_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Fract)),
            "ln_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Ln)),
            "ln1p_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Ln1p)),
            "log10_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Log10)),
            "log2_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Log2)),
            "recip_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Recip)),
            "round_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Round)),
            "signum_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Signum)),
            "sin_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Sin)),
            "sinh_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Sinh)),
            "sqrt_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Sqrt)),
            "tan_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Tan)),
            "tanh_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Tanh)),
            "trunc_double" => Ok(HostCall::UnaryOpF64(UnaryOp::Trunc)),
            "atan2_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Atan2)),
            "copysign_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Copysign)),
            "div_euclid_double" => Ok(HostCall::BinaryOpF64(BinaryOp::DivEuclid)),
            "hypot_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Hypot)),
            "log_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Log)),
            "max_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Max)),
            "min_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Min)),
            "powf_double" => Ok(HostCall::BinaryOpF64(BinaryOp::Powf)),
            "rem_euclid_double" => Ok(HostCall::BinaryOpF64(BinaryOp::RemEuclid)),
            _ => Err(()),
        }
    }

    pub fn signature(&self) -> (Vec<ValueType>, Option<ValueType>) {
        match self {
            HostCall::Upcall(UpcallId::Scan) => {
                (vec![ValueType::F32, ValueType::F32], Some(ValueType::I64))
            }
            HostCall::Upcall(UpcallId::Fire) => (vec![], None),
            HostCall::Upcall(UpcallId::Aim) => (vec![ValueType::F32], None),
            HostCall::Upcall(UpcallId::Turn) => (vec![ValueType::F32], None),
            HostCall::Upcall(UpcallId::GPSX) => (vec![], Some(ValueType::F32)),
            HostCall::Upcall(UpcallId::GPSY) => (vec![], Some(ValueType::F32)),
            HostCall::Upcall(UpcallId::Temp) => (vec![], Some(ValueType::I32)),
            HostCall::Upcall(UpcallId::Forward) => (vec![], None),
            HostCall::Upcall(UpcallId::Explode) => (vec![], None),
            HostCall::Upcall(UpcallId::Yield) => (vec![], None),
            HostCall::UnaryOpF32(_) => (vec![ValueType::F32], Some(ValueType::F32)),
            HostCall::BinaryOpF32(_) => {
                (vec![ValueType::F32, ValueType::F32], Some(ValueType::F32))
            }
            HostCall::UnaryOpF64(_) => (vec![ValueType::F64], Some(ValueType::F64)),
            HostCall::BinaryOpF64(_) => {
                (vec![ValueType::F64, ValueType::F64], Some(ValueType::F64))
            }
        }
    }

    pub fn from_id(id: usize) -> Option<Self> {
        match id {
            x @ B0..B1 => Some(HostCall::Upcall(FromPrimitive::from_usize(x - B0).unwrap())),
            x @ B1..B2 => Some(HostCall::UnaryOpF32(
                FromPrimitive::from_usize(x - B1).unwrap(),
            )),
            x @ B2..B3 => Some(HostCall::BinaryOpF32(
                FromPrimitive::from_usize(x - B2).unwrap(),
            )),
            x @ B3..B4 => Some(HostCall::UnaryOpF64(
                FromPrimitive::from_usize(x - B3).unwrap(),
            )),
            x @ B4..B5 => Some(HostCall::BinaryOpF64(
                FromPrimitive::from_usize(x - B4).unwrap(),
            )),
            _ => None,
        }
    }

    pub fn to_id(&self) -> usize {
        match self {
            HostCall::Upcall(x) => *x as usize,
            HostCall::UnaryOpF32(x) => *x as usize + NUM_UPCALLS,
            HostCall::BinaryOpF32(x) => *x as usize + NUM_UPCALLS + NUM_UNOPS,
            HostCall::UnaryOpF64(x) => *x as usize + NUM_UPCALLS + NUM_UNOPS + NUM_BINOPS,
            HostCall::BinaryOpF64(x) => *x as usize + NUM_UPCALLS + 2 * NUM_UNOPS + NUM_BINOPS,
        }
    }
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, FromPrimitive)]
enum UpcallId {
    Scan,
    Fire,
    Aim,
    Turn,
    GPSX,
    GPSY,
    Temp,
    Forward,
    Explode,
    Yield, // Must be last, or else change the constant below
}

const NUM_UPCALLS: usize = UpcallId::Yield as usize + 1;

#[repr(usize)]
#[derive(Clone, Copy, Debug, FromPrimitive)]
enum UnaryOp {
    Abs,
    Acos,
    Acosh,
    Asin,
    Asinh,
    Atan,
    Atanh,
    Cbrt,
    Ceil,
    Cos,
    Cosh,
    Exp,
    Exp2,
    ExpM1,
    Floor,
    Fract,
    Ln,
    Ln1p,
    Log10,
    Log2,
    Recip,
    Round,
    Signum,
    Sin,
    Sinh,
    Sqrt,
    Tan,
    Tanh,
    Trunc, // Must be last, or else change the constant below
}

const NUM_UNOPS: usize = UnaryOp::Trunc as usize + 1;

macro_rules! unary_body {
    ($match:ident, $var:ident) => {
        match &$match {
            UnaryOp::Abs => $var.abs(),
            UnaryOp::Acos => $var.acos(),
            UnaryOp::Acosh => $var.acosh(),
            UnaryOp::Asin => $var.asin(),
            UnaryOp::Asinh => $var.asinh(),
            UnaryOp::Atan => $var.atan(),
            UnaryOp::Atanh => $var.atanh(),
            UnaryOp::Cbrt => $var.cbrt(),
            UnaryOp::Ceil => $var.ceil(),
            UnaryOp::Cos => $var.cos(),
            UnaryOp::Cosh => $var.cosh(),
            UnaryOp::Exp => $var.exp(),
            UnaryOp::Exp2 => $var.exp2(),
            UnaryOp::ExpM1 => $var.exp_m1(),
            UnaryOp::Floor => $var.floor(),
            UnaryOp::Fract => $var.fract(),
            UnaryOp::Ln => $var.ln(),
            UnaryOp::Ln1p => $var.ln_1p(),
            UnaryOp::Log10 => $var.log10(),
            UnaryOp::Log2 => $var.log2(),
            UnaryOp::Recip => $var.recip(),
            UnaryOp::Round => $var.round(),
            UnaryOp::Signum => $var.signum(),
            UnaryOp::Sin => $var.sin(),
            UnaryOp::Sinh => $var.sinh(),
            UnaryOp::Sqrt => $var.sqrt(),
            UnaryOp::Tan => $var.tan(),
            UnaryOp::Tanh => $var.tanh(),
            UnaryOp::Trunc => $var.trunc(),
        }
    };
}

impl UnaryOp {
    pub fn do_f32(&self, x: f32) -> f32 {
        unary_body!(self, x)
    }

    pub fn do_f64(&self, x: f64) -> f64 {
        unary_body!(self, x)
    }

    pub fn do_runtime(&self, x: RuntimeValue) -> RuntimeValue {
        match x {
            RuntimeValue::F32(x) => RuntimeValue::F32(F32::from_float(self.do_f32(x.to_float()))),
            RuntimeValue::F64(x) => RuntimeValue::F64(F64::from_float(self.do_f64(x.to_float()))),
            _ => panic!("Attempt to do floating point operations on non-floating point types"),
        }
    }
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, FromPrimitive)]
enum BinaryOp {
    Atan2,
    Copysign,
    DivEuclid,
    Hypot,
    Log,
    Max,
    Min,
    Powf,
    RemEuclid, // Must be last, or else change the constant below
}

const NUM_BINOPS: usize = BinaryOp::RemEuclid as usize + 1;

macro_rules! binary_body {
    ($match:ident, $a:ident, $b:ident) => {
        match &$match {
            BinaryOp::Atan2 => $a.atan2($b),
            BinaryOp::Copysign => $a.copysign($b),
            BinaryOp::DivEuclid => $a.div_euclid($b),
            BinaryOp::Hypot => $a.hypot($b),
            BinaryOp::Log => $a.log($b),
            BinaryOp::Max => $a.max($b),
            BinaryOp::Min => $a.min($b),
            BinaryOp::Powf => $a.powf($b),
            BinaryOp::RemEuclid => $a.rem_euclid($b),
        }
    };
}

impl BinaryOp {
    pub fn do_f32(&self, a: f32, b: f32) -> f32 {
        binary_body!(self, a, b)
    }

    pub fn do_f64(&self, a: f64, b: f64) -> f64 {
        binary_body!(self, a, b)
    }

    pub fn do_runtime(&self, a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
        match (a, b) {
            (RuntimeValue::F32(a), RuntimeValue::F32(b)) => RuntimeValue::F32(F32::from_float(self.do_f32(a.to_float(), b.to_float()))),
            (RuntimeValue::F64(a), RuntimeValue::F64(b)) => RuntimeValue::F64(F64::from_float(self.do_f64(a.to_float(), b.to_float()))),
            _ => panic!("Attempt to do floating point operations on non-floating point or difform floating point types"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Upcall {
    None,
    Scan(f32, f32, Arc<Mutex<Option<u64>>>),
    Fire,
    Aim(f32),
    Turn(f32),
    GPSX(Arc<Mutex<Option<f32>>>),
    GPSY(Arc<Mutex<Option<f32>>>),
    Temp(Arc<Mutex<Option<i32>>>),
    Forward,
    Explode,
}

impl Upcall {
    pub fn alters_world(&self) -> bool {
        match self {
            Upcall::None => false,
            Upcall::Scan(_, _, _) => false,
            Upcall::Fire => true,
            Upcall::Aim(_) => true,
            Upcall::Turn(_) => true,
            Upcall::GPSX(_) => false,
            Upcall::GPSY(_) => false,
            Upcall::Temp(_) => false,
            Upcall::Forward => true,
            Upcall::Explode => true,
        }
    }
}

impl core::fmt::Display for Upcall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Upcall::None => write!(f, "none")?,
            Upcall::Scan(a, b, _) => write!(f, "scan between {} and {}", a, b)?,
            Upcall::Fire => write!(f, "fire")?,
            Upcall::Aim(h) => write!(f, "aim at {}", h)?,
            Upcall::Turn(h) => write!(f, "turn to {}", h)?,
            Upcall::GPSX(_) => write!(f, "get GPS X")?,
            Upcall::GPSY(_) => write!(f, "get GPS Y")?,
            Upcall::Temp(_) => write!(f, "get temperature")?,
            Upcall::Forward => write!(f, "move forward")?,
            Upcall::Explode => write!(f, "explode")?,
        }
        Ok(())
    }
}

impl HostError for Upcall {}

#[derive(Clone, Debug)]
struct HostFuncs {}

impl ModuleImportResolver for HostFuncs {
    fn resolve_func(
        &self,
        field_name: &str,
        signature: &Signature,
    ) -> Result<FuncRef, wasmi::Error> {
        let id = HostCall::from_name(field_name)
            .map_err(|_| wasmi::Error::Instantiation(format!("Export {} not found", field_name)))?;
        let (params, rt) = id.signature();
        if params != signature.params() || rt != signature.return_type() {
            return Err(wasmi::Error::Instantiation(format!(
                "Incorrect signature on {}",
                field_name
            )));
        }
        return Ok(FuncInstance::alloc_host(
            Signature::new(params, rt),
            id.to_id(),
        ));
    }
}

impl Externals for HostFuncs {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        let call = HostCall::from_id(index).ok_or(Trap::new(TrapKind::TableAccessOutOfBounds))?;
        //println!("call: {:?}", call);
        match call {
            HostCall::Upcall(uc) => Err(Trap::new(TrapKind::Host(Box::new(match uc {
                UpcallId::Scan => Upcall::Scan(
                    args.nth_checked::<F32>(0)?.to_float(),
                    args.nth_checked::<F32>(1)?.to_float(),
                    Arc::new(Mutex::new(None)),
                ),
                UpcallId::Fire => Upcall::Fire,
                UpcallId::Aim => Upcall::Aim(args.nth_checked::<F32>(0)?.to_float()),
                UpcallId::Turn => Upcall::Turn(args.nth_checked::<F32>(0)?.to_float()),
                UpcallId::GPSX => Upcall::GPSX(Arc::new(Mutex::new(None))),
                UpcallId::GPSY => Upcall::GPSY(Arc::new(Mutex::new(None))),
                UpcallId::Temp => Upcall::Temp(Arc::new(Mutex::new(None))),
                UpcallId::Forward => Upcall::Forward,
                UpcallId::Explode => Upcall::Explode,
                UpcallId::Yield => Upcall::None,
            })))),
            HostCall::UnaryOpF32(op) | HostCall::UnaryOpF64(op) => {
                Ok(Some(op.do_runtime(args.nth_value_checked(0)?)))
            }
            HostCall::BinaryOpF32(op) | HostCall::BinaryOpF64(op) => Ok(Some(
                op.do_runtime(args.nth_value_checked(0)?, args.nth_value_checked(1)?),
            )),
        }
    }
}

pub struct VM {
    wasm_func: Box<FuncInvocation<'static>>,
    externals: HostFuncs,
    state: VMState,
}

#[derive(Debug, Clone)]
enum VMState {
    Ready,
    Waiting(Upcall),
}

impl VM {
    pub fn new(program: Vec<u8>) -> Result<Self, wasmi::Error> {
        let mut externals = HostFuncs {};
        let module = wasmi::Module::from_buffer(&program)?;
        let instance = ModuleInstance::new(
            &module,
            &ImportsBuilder::new().with_resolver("env", &externals),
        )?;
        if let Some(ExternVal::Func(fr)) = instance.not_started_instance().export_by_name(&"tank") {
            let mut invocation = Box::new(FuncInstance::invoke_resumable(&fr, vec![])?);
            let result = invocation.start_execution_until(&mut externals, Some(0));
            loop {
                // Not a real loop, just something we can break out of
                if let Err(ResumableError::Trap(t)) = result {
                    if let TrapKind::TooManyInstructions = t.kind() {
                        break;
                    }
                }
                panic!("Invocation of WebAssembly failed before any steps were executed");
            }
            Ok(VM {
                wasm_func: invocation,
                externals,
                state: VMState::Ready,
            })
        } else {
            Err(wasmi::Error::Instantiation(
                "Entry point `tank` was not found".into(),
            ))
        }
    }

    pub fn begin_step(&mut self) {
        self.wasm_func.reset_counter();
    }

    pub fn counter(&self) -> isize {
        self.wasm_func.counter()
    }

    pub fn add_counter(&mut self, addend: isize) {
        self.wasm_func.add_counter(addend);
    }

    pub fn set_counter(&mut self, counter: isize) {
        self.wasm_func.set_counter(counter);
    }

    pub fn run_until(&mut self, max_count: Option<isize>) -> Upcall {
        let val = match &self.state {
            VMState::Ready => None,
            VMState::Waiting(Upcall::None) => None,
            VMState::Waiting(Upcall::Scan(_, _, v)) => Some(RuntimeValue::I64(i64::from_ne_bytes(
                (*v.lock().unwrap())
                    .expect("No value was returned by upcall")
                    .to_ne_bytes(),
            ))),
            VMState::Waiting(Upcall::Fire) => None,
            VMState::Waiting(Upcall::Aim(_)) => None,
            VMState::Waiting(Upcall::Turn(_)) => None,
            VMState::Waiting(Upcall::GPSX(v)) => Some(RuntimeValue::F32(F32::from_float(
                v.lock().unwrap().expect("No value was returned by upcall"),
            ))),
            VMState::Waiting(Upcall::GPSY(v)) => Some(RuntimeValue::F32(F32::from_float(
                v.lock().unwrap().expect("No value was returned by upcall"),
            ))),
            VMState::Waiting(Upcall::Temp(v)) => Some(RuntimeValue::I32(
                v.lock().unwrap().expect("No value was returned by upcall"),
            )),
            VMState::Waiting(Upcall::Forward) => None,
            VMState::Waiting(Upcall::Explode) => None,
        };
        //println!("running VM. state: {:?}. returned value: {:?}. expected value type: {:?}.", self.state, val, self.wasm_func.resumable_value_type());
        self.state = VMState::Ready;
        let result = self
            .wasm_func
            .resume_execution_until(val, &mut self.externals, max_count);
        match result {
            Err(ResumableError::Trap(t)) => match t.kind() {
                TrapKind::TooManyInstructions => Upcall::None,
                TrapKind::Host(h) => {
                    let uc = h.downcast_ref::<Upcall>().unwrap().clone();
                    self.state = VMState::Waiting(uc.clone());
                    uc
                }
                trap => {
                    println!("tank trapped: {:?}", trap);
                    Upcall::Explode
                }
            },
            err => {
                println!("tank finished or had an error: {:?}", err);
                Upcall::Explode
            }
        }
    }
}

// TODO: remove derive(Clone)s that necessitate this.
impl Clone for VM {
    fn clone(&self) -> Self {
        todo!("You cannot clone a VM yet -- need to remove derive(Clone)s that force an implementation at all");
    }
}

impl core::fmt::Debug for VM {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "opaque VM")?;
        Ok(())
    }
}

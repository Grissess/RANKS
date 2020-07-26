use std::sync::{Arc, Mutex};

use num_traits::FromPrimitive;
use num_derive::FromPrimitive;

use wasmi::{
    ExternVal, Externals, FuncInstance, FuncInvocation, FuncRef, HostError, ImportsBuilder,
    ModuleImportResolver, ModuleInstance, nan_preserving_float::F32, RuntimeArgs, RuntimeValue,
    Signature, Trap, TrapKind, ValueType, ResumableError
};

#[repr(usize)]
#[derive(FromPrimitive)]
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
    Yield,
}

impl UpcallId {
    pub fn from_name(name: &str) -> Result<Self, ()> {
        match name {
            "scan"      => Ok(UpcallId::Scan),
            "fire"      => Ok(UpcallId::Fire),
            "aim"       => Ok(UpcallId::Aim),
            "turn"      => Ok(UpcallId::Turn),
            "gpsx"      => Ok(UpcallId::GPSX),
            "gpsy"      => Ok(UpcallId::GPSY),
            "temp"      => Ok(UpcallId::Temp),
            "forward"   => Ok(UpcallId::Forward),
            "explode"   => Ok(UpcallId::Explode),
            "yield"     => Ok(UpcallId::Yield),
            _ => Err(()),
        }
    }

    pub fn signature(&self) -> (Vec<ValueType>, Option<ValueType>) {
        match self {
            UpcallId::Scan     => (vec![ValueType::F32, ValueType::F32], Some(ValueType::I64)),
            UpcallId::Fire     => (vec![], None),
            UpcallId::Aim      => (vec![ValueType::F32], None),
            UpcallId::Turn     => (vec![ValueType::F32], None),
            UpcallId::GPSX     => (vec![], Some(ValueType::F32)),
            UpcallId::GPSY     => (vec![], Some(ValueType::F32)),
            UpcallId::Temp     => (vec![], Some(ValueType::I32)),
            UpcallId::Forward  => (vec![], None),
            UpcallId::Explode  => (vec![], None),
            UpcallId::Yield    => (vec![], None),
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
            Upcall::None            => false,
            Upcall::Scan(_, _, _)   => false,
            Upcall::Fire            => true,
            Upcall::Aim(_)          => true,
            Upcall::Turn(_)         => true,
            Upcall::GPSX(_)         => false,
            Upcall::GPSY(_)         => false,
            Upcall::Temp(_)         => false,
            Upcall::Forward         => true,
            Upcall::Explode         => true,
        }
    }
}

impl core::fmt::Display for Upcall {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Upcall::None            => write!(f, "none")?,
            Upcall::Scan(a, b, _)   => write!(f, "scan between {} and {}", a, b)?,
            Upcall::Fire            => write!(f, "fire")?,
            Upcall::Aim(h)          => write!(f, "aim at {}", h)?,
            Upcall::Turn(h)         => write!(f, "turn to {}", h)?,
            Upcall::GPSX(_)         => write!(f, "get GPS X")?,
            Upcall::GPSY(_)         => write!(f, "get GPS Y")?,
            Upcall::Temp(_)         => write!(f, "get temperature")?,
            Upcall::Forward         => write!(f, "move forward")?,
            Upcall::Explode         => write!(f, "explode")?,
        }
        Ok(())
    }
}

impl HostError for Upcall {}

#[derive(Clone, Debug)]
struct Upcaller {}

impl ModuleImportResolver for Upcaller {
    fn resolve_func(
        &self,
        field_name: &str,
        signature: &Signature,
    ) -> Result<FuncRef, wasmi::Error> {
        let id = UpcallId::from_name(field_name).map_err(|_| wasmi::Error::Instantiation(format!("Export {} not found", field_name)))?;
        let (params, rt) = id.signature();
        if params != signature.params() || rt != signature.return_type() {
            return Err(wasmi::Error::Instantiation(format!("Incorrect signature on {}", field_name)));
        }
        return Ok(FuncInstance::alloc_host(Signature::new(params, rt), id as usize));
    }
}

impl Externals for Upcaller {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        Err(Trap::new(TrapKind::Host(Box::new(match FromPrimitive::from_usize(index).expect("Tried to invoke a function that doesn't exist!") {
            UpcallId::Scan => {
                Upcall::Scan(args.nth_checked::<F32>(0)?.to_float(), args.nth_checked::<F32>(1)?.to_float(), Arc::new(Mutex::new(None)))
            }
            UpcallId::Fire => {
                Upcall::Fire
            }
            UpcallId::Aim => {
                Upcall::Aim(args.nth_checked::<F32>(0)?.to_float())
            }
            UpcallId::Turn => {
                Upcall::Turn(args.nth_checked::<F32>(0)?.to_float())
            }
            UpcallId::GPSX => {
                Upcall::GPSX(Arc::new(Mutex::new(None)))
            }
            UpcallId::GPSY => {
                Upcall::GPSY(Arc::new(Mutex::new(None)))
            }
            UpcallId::Temp => {
                Upcall::Temp(Arc::new(Mutex::new(None)))
            }
            UpcallId::Forward => {
                Upcall::Forward
            }
            UpcallId::Explode => {
                Upcall::Explode
            }
            UpcallId::Yield => {
                Upcall::None
            }
        }))))
    }
}

pub struct VM {
    wasm_func: Box<FuncInvocation<'static>>,
    externals: Upcaller,
    state: VMState,
}

enum VMState {
    Ready,
    Waiting(Upcall),
}

impl VM {
    pub fn new(program: Vec<u8>) -> Result<Self, wasmi::Error> {
        let mut externals = Upcaller {};
        let module = wasmi::Module::from_buffer(&program)?;
        let instance = ModuleInstance::new(&module, &ImportsBuilder::new().with_resolver("env", &externals))?;
        if let Some(ExternVal::Func(fr)) = instance.not_started_instance().export_by_name(&"tank") {
            let mut invocation = Box::new(FuncInstance::invoke_resumable(&fr, vec![]).expect("failed to invoke function!"));
            let result = invocation.start_execution_until(&mut externals, Some(0));
            loop {  // Not a real loop, just something we can break out of
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
            Err(wasmi::Error::Instantiation("Entry point `tank` was not found".into()))
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
            VMState::Ready                           => None,
            VMState::Waiting(Upcall::None)           => None,
            VMState::Waiting(Upcall::Scan(_, _, v))  => Some(RuntimeValue::I64(i64::from_ne_bytes((*v.lock().unwrap()).expect("No value was returned by upcall").to_ne_bytes()))),
            VMState::Waiting(Upcall::Fire)           => None,
            VMState::Waiting(Upcall::Aim(_))         => None,
            VMState::Waiting(Upcall::Turn(_))        => None,
            VMState::Waiting(Upcall::GPSX(v))        => Some(RuntimeValue::F32(F32::from_float(v.lock().unwrap().expect("No value was returned by upcall")))),
            VMState::Waiting(Upcall::GPSY(v))        => Some(RuntimeValue::F32(F32::from_float(v.lock().unwrap().expect("No value was returned by upcall")))),
            VMState::Waiting(Upcall::Temp(v))        => Some(RuntimeValue::I32(v.lock().unwrap().expect("No value was returned by upcall"))),
            VMState::Waiting(Upcall::Forward)        => None,
            VMState::Waiting(Upcall::Explode)        => None,
        };
        let result = self.wasm_func.resume_execution_until(val, &mut self.externals, max_count);
        match result {
            Err(ResumableError::Trap(t))  => {
                match t.kind() {
                    TrapKind::TooManyInstructions => {
                        Upcall::None
                    },
                    TrapKind::Host(h) => {
                        let uc = h.downcast_ref::<Upcall>().unwrap().clone();
                        self.state = VMState::Waiting(uc.clone());
                        uc
                    }
                    _ => Upcall::Explode,
                }
            },
            _ => Upcall::Explode,
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


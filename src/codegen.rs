

use inkwell::AddressSpace::Global;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Linkage;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicMetadataValueEnum, IntValue};
use inkwell::{OptimizationLevel};
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use crate::parser::Expr;

pub struct Compiler<'a, 'ctx> {
    pub context: &'ctx Context,
    pub builder: &'a Builder<'ctx>,
    pub module: &'a Module<'ctx>,
    pub variables: Vec<IntValue<'ctx>>,
    pub execution_engine: &'a ExecutionEngine<'ctx>,
    pub printf_defined: bool,
    pub string_count: u16,
    pub print_stack_count: u16
}

impl<'a, 'ctx> Compiler<'a, 'ctx> {

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<IntValue<'ctx>, &'static str> {
        match &expr {
            Expr::Number(nb) => {
                let return_val = self.context.i32_type().const_int(nb.n as u64, true);
                self.variables.push(return_val.clone());
                Ok(return_val)
            },
            Expr::Binary { op, ref left, ref right } => {
                let lhs = self.compile_expr(left)?;
                let rhs = self.compile_expr(right)?;
                self.variables.pop();
                self.variables.pop();
                match op {
                    '+' => {
                        let return_val = self.builder.build_int_add(lhs, rhs, "anAdd");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    '-' => {
                        let return_val = self.builder.build_int_sub(lhs, rhs, "aSub");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    '*' => {
                        let return_val = self.builder.build_int_mul(lhs, rhs, "aMult");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    '/' => {
                        let return_val = self.builder.build_int_signed_div(lhs, rhs, "aDiv");
                        self.variables.push(return_val.clone());
                        Ok(return_val)
                    },
                    _ => Err("Invalid operator. Check parser did not parse incorrectly.")
                }
            }
            Expr::Call { ref other, ref actual } => {
                let some_expr = self.compile_expr(other);
                let _ = self.compile_expr(actual);
                Ok(some_expr.unwrap())
            }
            Expr::StringPrint(str) => {
                if !self.printf_defined {
                    self.define_printf();
                    self.printf_defined = true
                }
                let name_of_string = "string".to_string() + &self.string_count.to_string();
                let the_string = self.builder.build_global_string_ptr(str.as_str(), name_of_string.as_str());
                self.string_count += 1;
                let mut arguments: Vec<BasicMetadataValueEnum> = vec![];
                arguments.push(the_string.as_pointer_value().into());
                self.builder.build_call(self.module.get_function("printf").unwrap(), &arguments, "printf");
                Ok(self.context.i32_type().const_int(0, false))
            }
            Expr::PrintStack => {
                if !self.printf_defined {
                    self.define_printf();
                    self.printf_defined = true
                }
                let mut arguments: Vec<BasicMetadataValueEnum> = vec![];
                let mut format_string: String = "".to_string();
                let name_of_string = "print_stack".to_string() + &self.print_stack_count.to_string();
                for _ in self.variables.clone() {
                    format_string = format_string + "%d ";
                }
                let the_string = self.builder.build_global_string_ptr(format_string.as_str(), name_of_string.as_str());
                self.print_stack_count += 1;
                arguments.push(the_string.as_pointer_value().into());
                for var in self.variables.clone() {
                    arguments.push(var.into());
                }

                self.builder.build_call(self.module.get_function("printf").unwrap(), &arguments, "printf");
                Ok(self.context.i32_type().const_int(0, false))
            }
            Expr::ProgramEnd => {
                Ok(self.context.i32_type().const_int(0, false))
            }
        }
    }

    pub fn define_printf(&self) {
        let printf_fn_type = self.context.i32_type().fn_type(&[self.context.ptr_sized_int_type(self.execution_engine.get_target_data(), Option::from(Global)).into()], true);
        self.module.add_function("printf", printf_fn_type, Some(Linkage::External)).set_call_conventions(0); // https://llvm.org/doxygen/namespacellvm_1_1CallingConv.html
    }

    pub fn build_main(&self) {
        let main_fn_type = self.context.i32_type().fn_type(&[], false);
        let main_fn = self.module.add_function("main", main_fn_type, Some(Linkage::External));
        let basic_block = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(basic_block);
    }

    pub fn build_end_return(&self) {
        let i32_type = self.context.i32_type();
        let i32_zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&i32_zero));
    }

    pub fn write_to_file(&self) -> Result<(), String> {
        Target::initialize_all(&InitializationConfig::default());
        let target_triple = TargetMachine::get_default_triple();
        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();

        let target = Target::from_triple(&target_triple).map_err(|e| format!("{:?}", e))?;
        let target_machine = target
            .create_target_machine(
                &target_triple,
                &cpu,
                &features,
                OptimizationLevel::Default,
                RelocMode::DynamicNoPic,
                CodeModel::Default,
            )
            .ok_or_else(|| "Unable to create target machine!".to_string())?;

        let buff = target_machine
            .write_to_memory_buffer(&self.module, FileType::Assembly)
            .expect("couldn't compile to assembly");

        println!(
            "Assembly:\n{}",
            String::from_utf8(buff.as_slice().to_vec()).unwrap()
        );

        target_machine
            .write_to_file(&self.module, FileType::Object, "a.o".as_ref())
            .map_err(|e| format!("{:?}", e))
    }
}
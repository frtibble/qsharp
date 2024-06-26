// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use qsc_data_structures::functors::FunctorApp;
use qsc_eval::{
    val::{Result, Value},
    Env, Variable,
};
use qsc_fir::fir::{LocalItemId, LocalVarId, PackageId};
use qsc_rca::{RuntimeKind, ValueKind};
use qsc_rir::rir::BlockId;
use rustc_hash::FxHashMap;

/// Struct that keeps track of the active RIR blocks (where RIR instructions are added) and the active scopes (which
/// correspond to the Q#'s program call stack).
pub struct EvaluationContext {
    active_blocks: Vec<BlockNode>,
    scopes: Vec<Scope>,
}

impl EvaluationContext {
    /// Creates a new evaluation context.
    pub fn new(package_id: PackageId, initial_block: BlockId) -> Self {
        let entry_callable_scope = Scope::new(package_id, None, Vec::new());
        Self {
            active_blocks: vec![BlockNode {
                id: initial_block,
                successor: None,
            }],
            scopes: vec![entry_callable_scope],
        }
    }

    /// Gets the ID of the current RIR block.
    pub fn get_current_block_id(&self) -> BlockId {
        self.active_blocks.last().expect("no active blocks").id
    }

    /// Gets an immutable reference to the current (call) scope.
    pub fn get_current_scope(&self) -> &Scope {
        self.scopes
            .last()
            .expect("the evaluation context does not have a current scope")
    }

    /// Gets a mutable reference to the current (call) scope.
    pub fn get_current_scope_mut(&mut self) -> &mut Scope {
        self.scopes
            .last_mut()
            .expect("the evaluation context does not have a current scope")
    }

    /// Pops the currently active block.
    pub fn pop_block_node(&mut self) -> BlockNode {
        self.get_current_scope_mut().active_block_count -= 1;
        self.active_blocks
            .pop()
            .expect("there are no active blocks in the evaluation context")
    }

    /// Pops the currently active (call) scope.
    pub fn pop_scope(&mut self) -> Scope {
        self.scopes
            .pop()
            .expect("there are no scopes in the evaluation context")
    }

    /// Pushes a new active block.
    pub fn push_block_node(&mut self, b: BlockNode) {
        self.active_blocks.push(b);
        self.get_current_scope_mut().active_block_count += 1;
    }

    /// Pushes a new (call) scope.
    pub fn push_scope(&mut self, s: Scope) {
        self.scopes.push(s);
    }
}

/// Struct that represents a block node when we intepret an RIR program as a graph.
pub struct BlockNode {
    /// The ID of the block.
    pub id: BlockId,
    /// The block to jump to (if any) once all instructions to the block have been added.
    pub successor: Option<BlockId>,
}

/// A call scope.
pub struct Scope {
    /// The package ID of the callable.
    pub package_id: PackageId,
    /// The ID and functor information of the callable.
    pub callable: Option<(LocalItemId, FunctorApp)>,
    /// The value of the arguments passed to the callable.
    pub args_value_kind: Vec<ValueKind>,
    /// The classical environment of the callable, which holds values corresponding to local variables.
    pub env: Env,
    /// Number of currently active blocks (starting from where this scope was created).
    active_block_count: usize,
    /// Map that holds the values of local variables.
    hybrid_vars: FxHashMap<LocalVarId, Value>,
}

impl Scope {
    /// Creates a new call scope.
    pub fn new(
        package_id: PackageId,
        callable: Option<(LocalItemId, FunctorApp)>,
        args: Vec<Arg>,
    ) -> Self {
        // Create the environment for the classical evaluator.
        // A default classical evaluator environment is created with one scope. However, we need to push an additional
        // scope to the environment to be able to detect whether the classical evaluator has returned from the call
        // scope.
        const CLASSICAL_EVALUATOR_CALL_SCOPE_ID: usize = 1;
        let mut env = Env::default();
        env.push_scope(CLASSICAL_EVALUATOR_CALL_SCOPE_ID);

        // Determine the runtime kind (static or dynamic) of the arguments.
        let args_value_kind: Vec<ValueKind> = args
            .iter()
            .map(|arg| {
                let value = match arg {
                    Arg::Discard(value) => value,
                    Arg::Var(_, var) => &var.value,
                };
                map_eval_value_to_value_kind(value)
            })
            .collect();

        // Add the values to either the environment or the hybrid variables depending on whether the value is static or
        // dynamic.
        let mut hybrid_vars = FxHashMap::default();
        let arg_runtime_kind_tuple = args.into_iter().zip(args_value_kind.iter());
        for (arg, value_kind) in arg_runtime_kind_tuple {
            let Arg::Var(local_var_id, var) = arg else {
                continue;
            };

            if value_kind.is_dynamic() {
                hybrid_vars.insert(local_var_id, var.value);
            } else {
                env.bind_variable_in_top_frame(local_var_id, var);
            }
        }

        // Add the dynamic values to the hybrid variables
        Self {
            package_id,
            callable,
            args_value_kind,
            env,
            active_block_count: 1,
            hybrid_vars,
        }
    }

    /// Gets the value of a (hybrid) local variable.
    pub fn get_local_var_value(&self, local_var_id: LocalVarId) -> &Value {
        self.hybrid_vars
            .get(&local_var_id)
            .expect("local variable value does not exist")
    }

    /// Determines whether we are currently evaluating a branch within the scope.
    pub fn is_currently_evaluating_branch(&self) -> bool {
        self.active_block_count > 1
    }

    /// Determines whether the classical evaluator has returned from the call scope.
    /// This relies on the fact that the classical evaluator pops the scope when it encounters a return, so when this
    /// happens the number of scopes in the environment will be exactly one.
    pub fn has_classical_evaluator_returned(&self) -> bool {
        self.env.len() == 1
    }

    /// Inserts the value of a local variable into the hybrid variables map.
    pub fn insert_local_var_value(&mut self, local_var_id: LocalVarId, value: Value) {
        self.hybrid_vars.insert(local_var_id, value);
    }
}

/// A call argument.
pub enum Arg {
    Discard(Value),
    Var(LocalVarId, Variable),
}

impl Arg {
    /// Converts the argument into its underlying value.
    pub fn into_value(self) -> Value {
        match self {
            Self::Discard(value) => value,
            Self::Var(_, var) => var.value,
        }
    }
}

/// Represents the possible control flow options that can result from a branch.
pub enum BranchControlFlow {
    /// The block ID corresponding to a branch.
    Block(BlockId),
    /// The return value resulting from a branch.
    Return(Value),
}

/// Represents the possible control flow options that an evaluation can have.
pub enum EvalControlFlow {
    Continue(Value),
    Return(Value),
}

impl EvalControlFlow {
    /// Consumes the evaluation control flow and returns its value.
    pub fn into_value(self) -> Value {
        match self {
            EvalControlFlow::Continue(value) | EvalControlFlow::Return(value) => value,
        }
    }

    /// Whether this evaluation control flow is a return.
    pub fn is_return(&self) -> bool {
        match self {
            Self::Continue(_) => false,
            Self::Return(_) => true,
        }
    }
}

fn map_eval_value_to_value_kind(value: &Value) -> ValueKind {
    fn map_array_eval_value_to_value_kind(elements: &[Value]) -> ValueKind {
        let mut content_runtime_kind = RuntimeKind::Static;
        for element in elements {
            let element_value_kind = map_eval_value_to_value_kind(element);
            if element_value_kind.is_dynamic() {
                content_runtime_kind = RuntimeKind::Dynamic;
                break;
            }
        }

        // The runtime capabilities check pass disallows dynamically-sized arrays for all targets for which we generate
        // QIR. Because of this, we assume that during partial evaluation all arrays are statically-sized.
        ValueKind::Array(content_runtime_kind, RuntimeKind::Static)
    }

    fn map_tuple_eval_value_to_value_kind(elements: &[Value]) -> ValueKind {
        let mut runtime_kind = RuntimeKind::Static;
        for element in elements {
            let element_value_kind = map_eval_value_to_value_kind(element);
            if element_value_kind.is_dynamic() {
                runtime_kind = RuntimeKind::Dynamic;
                break;
            }
        }
        ValueKind::Element(runtime_kind)
    }

    match value {
        Value::Array(elements) => map_array_eval_value_to_value_kind(elements),
        Value::Tuple(elements) => map_tuple_eval_value_to_value_kind(elements),
        Value::Result(Result::Id(_)) | Value::Var(_) => ValueKind::Element(RuntimeKind::Dynamic),
        Value::BigInt(_)
        | Value::Bool(_)
        | Value::Closure(_)
        | Value::Double(_)
        | Value::Global(_, _)
        | Value::Int(_)
        | Value::Pauli(_)
        | Value::Qubit(_)
        | Value::Range(_)
        | Value::Result(Result::Val(_))
        | Value::String(_) => ValueKind::Element(RuntimeKind::Static),
    }
}

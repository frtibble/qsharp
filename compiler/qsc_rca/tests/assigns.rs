// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

#![allow(clippy::needless_raw_string_hashes)]

pub mod test_utils;

use expect_test::expect;
use test_utils::{check_last_statement_compute_properties, CompilationContext};

#[test]
fn check_rca_for_classical_int_assign_to_local() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        mutable i = 0;
        set i = 1;
        i"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGeneratorSet:
                inherent: Classical
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_dynamic_result_assign_to_local() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        use q = Qubit();
        mutable r = Zero;
        set r = M(q);
        r"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGeneratorSet:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(0x0)
                    value_kind: Element(Dynamic)
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_dynamic_bool_assign_to_local() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        use q = Qubit();
        mutable b = false;
        set b = ResultAsBool(M(q));
        b"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGeneratorSet:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool)
                    value_kind: Element(Dynamic)
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_dynamic_int_assign_to_local() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        open Microsoft.Quantum.Measurement;
        use register = Qubit[8];
        let results = MeasureEachZ(register);
        mutable i = 0;
        set i = ResultArrayAsInt(results);
        i"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGeneratorSet:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt)
                    value_kind: Element(Dynamic)
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn check_rca_for_dynamic_double_assign_to_local() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        open Microsoft.Quantum.Convert;
        open Microsoft.Quantum.Measurement;
        use register = Qubit[8];
        let results = MeasureEachZ(register);
        let i = ResultArrayAsInt(results);
        mutable d = 0.0;
        set d = IntAsDouble(i);
        d"#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![
            r#"
            ApplicationsGeneratorSet:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt | UseOfDynamicDouble)
                    value_kind: Element(Dynamic)
                dynamic_param_applications: <empty>"#
        ],
    );
}

#[test]
fn chec_rca_for_assign_call_result_to_tuple_of_vars() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        function Foo() : (Int, Int) {
            return (1,2);
        }
        mutable a = 1;
        mutable b = 2;
        set (a, b) = Foo();
        "#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Classical
                dynamic_param_applications: <empty>"#]],
    );
}

#[test]
fn chec_rca_for_assign_var_binded_to_call_result_to_tuple_of_vars() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        function Foo() : (Int, Int) {
            return (1,2);
        }
        let x = Foo();
        mutable a = 1;
        mutable b = 2;
        set (a, b) = x;
        "#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Classical
                dynamic_param_applications: <empty>"#]],
    );
}

#[test]
fn chec_rca_for_assign_tuple_var_to_tuple_of_vars() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        let x = (1, (2, 3));
        mutable a = 4;
        mutable b = (5, 6);
        set (a, b) = x;
        "#,
    );
    let package_store_compute_properties = compilation_context.get_compute_properties();
    check_last_statement_compute_properties(
        package_store_compute_properties,
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Classical
                dynamic_param_applications: <empty>"#]],
    );
}

#[test]
fn check_rca_for_assign_classical_call_result_to_tuple_of_vars() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        function Foo(a : Int, b : Int) : (Int, Int) {
            return (b, a);
        }
        mutable a = 1;
        mutable b = 2;
        set (a, b) = Foo(a, b);
        a
        "#,
    );
    check_last_statement_compute_properties(
        compilation_context.get_compute_properties(),
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Classical
                dynamic_param_applications: <empty>"#]],
    );
    compilation_context.update(
        r#"
        b
        "#,
    );
    check_last_statement_compute_properties(
        compilation_context.get_compute_properties(),
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Classical
                dynamic_param_applications: <empty>"#]],
    );
}

#[test]
fn check_rca_for_assign_dynamic_call_result_to_tuple_of_vars() {
    let mut compilation_context = CompilationContext::default();
    compilation_context.update(
        r#"
        function Foo(a : Int, b : Int) : (Int, Int) {
            return (b, a);
        }
        use q = Qubit();
        let r = MResetZ(q);
        mutable a = r == Zero ? 0 | 1;
        mutable b = 2;
        set (a, b) = Foo(a, b);
        a
        "#,
    );
    check_last_statement_compute_properties(
        compilation_context.get_compute_properties(),
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt)
                    value_kind: Element(Dynamic)
                dynamic_param_applications: <empty>"#]],
    );
    compilation_context.update(
        r#"
        b
        "#,
    );
    check_last_statement_compute_properties(
        compilation_context.get_compute_properties(),
        &expect![[r#"
            ApplicationsGeneratorSet:
                inherent: Quantum: QuantumProperties:
                    runtime_features: RuntimeFeatureFlags(UseOfDynamicBool | UseOfDynamicInt)
                    value_kind: Element(Dynamic)
                dynamic_param_applications: <empty>"#]],
    );
}

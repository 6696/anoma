use std::collections::HashSet;

use anoma_shared::ledger::gas::VpGasMeter;
use anoma_shared::ledger::storage::mockdb::MockDB;
use anoma_shared::ledger::storage::testing::TestStorage;
use anoma_shared::ledger::storage::write_log::WriteLog;
use anoma_shared::proto::Tx;
use anoma_shared::types::address::{self, Address};
use anoma_shared::types::storage::{self, Key};
use anoma_shared::vm;
use anoma_shared::vm::prefix_iter::PrefixIterators;

use crate::tx::{init_tx_env, TestTxEnv};

/// This module combines the native host function implementations from
/// `native_vp_host_env` with the functions exposed to the vp wasm
/// that will call to the native functions, instead of interfacing via a
/// wasm runtime. It can be used for host environment integration tests.
pub mod vp_host_env {
    pub use anoma_vm_env::imports::vp::*;

    pub use super::native_vp_host_env::*;
}

/// Host environment structures required for transactions.
pub struct TestVpEnv {
    pub addr: Address,
    pub storage: TestStorage,
    pub write_log: WriteLog,
    pub iterators: PrefixIterators<'static, MockDB>,
    pub gas_meter: VpGasMeter,
    pub tx: Tx,
    pub keys_changed: HashSet<storage::Key>,
    pub verifiers: HashSet<Address>,
    pub eval_runner: native_vp_host_env::VpEval,
    pub result_buffer: Option<Vec<u8>>,
}

impl Default for TestVpEnv {
    fn default() -> Self {
        #[cfg(feature = "wasm-runtime")]
        let eval_runner = anoma_shared::vm::wasm::run::VpEvalWasm::default();
        #[cfg(not(feature = "wasm-runtime"))]
        let eval_runner = native_vp_host_env::VpEval;

        Self {
            addr: address::testing::established_address_1(),
            storage: TestStorage::default(),
            write_log: WriteLog::default(),
            iterators: PrefixIterators::default(),
            gas_meter: VpGasMeter::new(0),
            tx: Tx::new(vec![], None),
            keys_changed: HashSet::default(),
            verifiers: HashSet::default(),
            eval_runner,
            result_buffer: None,
        }
    }
}

/// Initialize the host environment inside the [`vp_host_env`] module by running
/// a transaction. The transaction is expected to modify the given address
/// `addr` or to add it to the set of verifiers using
/// [`super::tx::tx_host_env::insert_verifier`].
pub fn init_vp_env_from_tx<F>(
    addr: Address,
    mut tx_env: TestTxEnv,
    mut apply_tx: F,
) -> TestVpEnv
where
    F: FnMut(&Address),
{
    // Write an empty validity predicate for the address, because it's used to
    // check if the address exists when we write into its storage
    let vp_key = Key::validity_predicate(&addr);
    tx_env.storage.write(&vp_key, vec![]).unwrap();

    init_tx_env(&mut tx_env);
    apply_tx(&addr);

    let verifiers_from_tx = &tx_env.verifiers;
    let verifiers_changed_keys =
        tx_env.write_log.verifiers_changed_keys(verifiers_from_tx);
    let verifiers = verifiers_changed_keys.keys().cloned().collect();
    let keys_changed = verifiers_changed_keys
        .get(&addr)
        .expect(
            "The VP for the given address has not been triggered by the \
             transaction",
        )
        .to_owned();

    let mut vp_env = TestVpEnv {
        addr,
        storage: tx_env.storage,
        write_log: tx_env.write_log,
        keys_changed,
        verifiers,
        ..Default::default()
    };

    init_vp_env(&mut vp_env);
    vp_env
}

/// Initialize the host environment inside the [`vp_host_env`] module.
pub fn init_vp_env(
    TestVpEnv {
        addr,
        storage,
        write_log,
        iterators,
        gas_meter,
        tx,
        keys_changed,
        verifiers,
        eval_runner,
        result_buffer,
    }: &mut TestVpEnv,
) {
    vp_host_env::ENV.with(|env| {
        *env.borrow_mut() = Some({
            vm::host_env::testing::vp_env(
                addr,
                storage,
                write_log,
                iterators,
                gas_meter,
                tx,
                verifiers,
                result_buffer,
                keys_changed,
                eval_runner,
            )
        })
    });
}

/// This module allows to test code with vp host environment functions.
/// It keeps a thread-local global `VpEnv`, which is passed to any of
/// invoked host environment functions and so it must be initialized
/// before the test.
mod native_vp_host_env {

    use std::cell::RefCell;

    use anoma_shared::ledger::storage::testing::Sha256Hasher;
    use anoma_shared::vm::host_env::*;
    use anoma_shared::vm::memory::testing::NativeMemory;
    // TODO replace with `std::concat_idents` once stabilized (https://github.com/rust-lang/rust/issues/29599)
    use concat_idents::concat_idents;

    use super::*;

    #[cfg(feature = "wasm-runtime")]
    pub type VpEval =
        anoma_shared::vm::wasm::run::VpEvalWasm<MockDB, Sha256Hasher>;
    #[cfg(not(feature = "wasm-runtime"))]
    pub struct VpEval;

    thread_local! {
        pub static ENV: RefCell<Option<VpEnv<'static, NativeMemory, MockDB, Sha256Hasher, VpEval>>> = RefCell::new(None);
    }

    #[cfg(not(feature = "wasm-runtime"))]
    impl VpEvaluator for VpEval {
        type Db = MockDB;
        type Eval = VpEval;
        type H = Sha256Hasher;

        fn eval(
            &self,
            _ctx: VpCtx<'static, Self::Db, Self::H, Self::Eval>,
            _vp_code: Vec<u8>,
            _input_data: Vec<u8>,
        ) -> anoma_shared::types::internal::HostEnvResult {
            unimplemented!(
                "The \"wasm-runtime\" feature must be enabled to test with \
                 the `eval` function."
            )
        }
    }

    /// A helper macro to create implementations of the host environment
    /// functions exported to wasm, which uses the environment from the
    /// `ENV` variable.
    macro_rules! native_host_fn {
            // unit return type
            ( $fn:ident ( $($arg:ident : $type:ty),* $(,)?) ) => {
                concat_idents!(extern_fn_name = anoma, _, $fn {
                    #[no_mangle]
                    extern "C" fn extern_fn_name( $($arg: $type),* ) {
                        ENV.with(|env| {
                            let env = env.borrow_mut();
                            let env = env.as_ref().expect("Did you forget to
    initialize the ENV?");

                            // Call the `host_env` function and unwrap any
                            // runtime errors
                            $fn( &env, $($arg),* ).unwrap()
                        })
                    }
                });
            };

            // non-unit return type
            ( $fn:ident ( $($arg:ident : $type:ty),* $(,)?) -> $ret:ty ) => {
                concat_idents!(extern_fn_name = anoma, _, $fn {
                    #[no_mangle]
                    extern "C" fn extern_fn_name( $($arg: $type),* ) -> $ret {
                        ENV.with(|env| {
                            let env = env.borrow_mut();
                            let env = env.as_ref().expect("Did you forget to
    initialize the ENV?");

                            // Call the `host_env` function and unwrap any
                            // runtime errors
                            $fn( &env, $($arg),* ).unwrap()
                        })
                    }
                });
            }
        }

    // Implement all the exported functions from
    // [`anoma_vm_env::imports::vp`] `extern "C"` section.
    native_host_fn!(vp_read_pre(key_ptr: u64, key_len: u64) -> i64);
    native_host_fn!(vp_read_post(key_ptr: u64, key_len: u64) -> i64);
    native_host_fn!(vp_result_buffer(result_ptr: u64));
    native_host_fn!(vp_has_key_pre(key_ptr: u64, key_len: u64) -> i64);
    native_host_fn!(vp_has_key_post(key_ptr: u64, key_len: u64) -> i64);
    native_host_fn!(vp_iter_prefix(prefix_ptr: u64, prefix_len: u64) -> u64);
    native_host_fn!(vp_iter_pre_next(iter_id: u64) -> i64);
    native_host_fn!(vp_iter_post_next(iter_id: u64) -> i64);
    native_host_fn!(vp_get_chain_id(result_ptr: u64));
    native_host_fn!(vp_get_block_height() -> u64);
    native_host_fn!(vp_get_block_hash(result_ptr: u64));
    native_host_fn!(vp_get_block_epoch() -> u64);
    native_host_fn!(vp_verify_tx_signature(
            pk_ptr: u64,
            pk_len: u64,
            sig_ptr: u64,
            sig_len: u64,
        ) -> i64);
    native_host_fn!(vp_eval(
            vp_code_ptr: u64,
            vp_code_len: u64,
            input_data_ptr: u64,
            input_data_len: u64,
        ) -> i64);
    native_host_fn!(vp_log_string(str_ptr: u64, str_len: u64));
}

#![allow(missing_docs)]

#[global_allocator]
static ALLOC: reth_cli_util::allocator::Allocator = reth_cli_util::allocator::new_allocator();

#[cfg(all(feature = "jemalloc-prof", unix))]
#[unsafe(export_name = "_rjem_malloc_conf")]
static MALLOC_CONF: &[u8] = b"prof:true,prof_active:true,lg_prof_sample:19\0";

use std::sync::Arc;

use clap::Parser;
use ew_exex::{db::RethDb as WitnessIndexerDb, exex::{WitnessIndexer, WitnessIndexerConfig}, rpc::{IndexedWitnessRpcApiServer, WitnessServiceRpc}};
use reth::{args::RessArgs, cli::Cli, ress::install_ress_subprotocol};
use reth_ethereum_cli::chainspec::EthereumChainSpecParser;
use reth_node_builder::NodeHandle;
use reth_node_ethereum::EthereumNode;
use tracing::info;

fn main() {
    reth_cli_util::sigsegv_handler::install();

    // Enable backtraces unless a RUST_BACKTRACE value has already been explicitly provided.
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }

    let indexer_config = WitnessIndexerConfig::default();

    if let Err(err) =
        Cli::<EthereumChainSpecParser, RessArgs>::parse().run(async move |builder, ress_args| {
            info!(target: "reth::cli", "Launching node");

            let db_path = builder
                .config()
                .datadir()
                .data_dir()
                .join("witness-indexer");
            let db_args = reth_db::mdbx::DatabaseArguments::default();
            let db_for_exex = Arc::new(WitnessIndexerDb::new_with_opts(db_path, db_args)?);
            let db_for_rpc = Arc::clone(&db_for_exex);



            let NodeHandle { node, node_exit_future } =
                builder
                .node(EthereumNode::default())
                .extend_rpc_modules(move |ctx| {
                    let rpc = WitnessServiceRpc::new(ctx.provider().clone(), db_for_rpc);
                    ctx.modules.merge_configured(rpc.into_rpc())?;
                    Ok(())
                })
                .install_exex("reth-witness-indexer", |ctx| async move {
                    let indexer =
                        WitnessIndexer::new(ctx, indexer_config.with_db(db_for_exex)).await?;
                    Ok(indexer.run())
                })
                .launch_with_debug_capabilities()
                .await?;

            // Install ress subprotocol.
            if ress_args.enabled {
                install_ress_subprotocol(
                    ress_args,
                    node.provider,
                    node.evm_config,
                    node.network,
                    node.task_executor,
                    node.add_ons_handle.engine_events.new_listener(),
                )?;
            }

            node_exit_future.await
        })
    {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

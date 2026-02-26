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

#[derive(Debug, Clone, clap::Args)]
pub struct ExtArgs {
    #[command(flatten)]
    pub ress: RessArgs,

    #[command(flatten)]
    pub indexer: WitnessIndexerConfig,
}

fn main() {
    reth_cli_util::sigsegv_handler::install();

    // Enable backtraces unless a RUST_BACKTRACE value has already been explicitly provided.
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }

    if let Err(err) =
        Cli::<EthereumChainSpecParser, ExtArgs>::parse().run(async move |builder, ext_args| {
            info!(target: "reth::cli", "Launching node");

            let db_path = builder
                .config()
                .datadir()
                .data_dir()
                .join("witness-indexer");
            let db_args = reth_db::mdbx::DatabaseArguments::default();
            let db_for_exex = Arc::new(WitnessIndexerDb::new_with_opts(&db_path, db_args)?);
            let db_for_rpc = Arc::clone(&db_for_exex);

            // Use configured directory or default fallback
            let witness_dir = ext_args
                .indexer
                .witness_dir
                .clone()
                .unwrap_or_else(|| db_path.parent().unwrap().join("witness-files"));

            let indexer_config = ext_args.indexer;
            
            let NodeHandle { node, node_exit_future } =
                builder
                .node(EthereumNode::default())
                .extend_rpc_modules(move |ctx| {
                    let rpc = WitnessServiceRpc::new(ctx.provider().clone(), db_for_rpc);
                    ctx.modules.merge_configured(rpc.into_rpc())?;
                    Ok(())
                })
                .install_exex("reth-witness-indexer", {
                    let witness_dir_clone = witness_dir.clone();
                    move |ctx| async move {
                        let config = indexer_config
                            .with_db(db_for_exex)
                            .with_witness_dir(witness_dir_clone);
                        let indexer = WitnessIndexer::new(ctx, config).await?;
                        Ok(indexer.run())
                    }
                })
                .launch_with_debug_capabilities()
                .await?;

            let tcp_server = Arc::new(ew_exex::tcp::WitnessServiceTcp::new(
                node.provider.clone(),
                witness_dir,
            ));
            tokio::spawn(async move {
                if let Err(e) = tcp_server.run_server("127.0.0.1:8005").await {
                    tracing::error!("TCP server failed: {}", e);
                }
            });

            // Install ress subprotocol.
            if ext_args.ress.enabled {
                install_ress_subprotocol(
                    ext_args.ress,
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

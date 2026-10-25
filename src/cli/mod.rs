//! Command line front end.

mod render;

use crate::chain::{Ledger, Transaction, UtxoSet};
use crate::error::{err, Error, Result};
use crate::net::{self, Peer};
use crate::store::Store;
use crate::wallet::{decode_address, Keystore};
use clap::{Arg, ArgMatches, Command};

pub fn run() -> Result<()> {
    let matches = command_tree().get_matches();
    match matches.subcommand() {
        Some(("wallet", sub)) => run_wallet(sub),
        Some(("chain", sub)) => run_chain(sub),
        Some(("balance", sub)) => run_balance(sub),
        Some(("send", sub)) => run_send(sub),
        Some(("utxo", sub)) => run_utxo(sub),
        Some(("node", sub)) => run_node(sub),
        _ => err("unknown command"),
    }
}

fn positional(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name).required(true).help(help)
}

fn command_tree() -> Command {
    Command::new("minichain")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A small educational blockchain with wallets, a UTXO ledger and proof-of-work")
        .subcommand_required(true)
        .subcommand(
            Command::new("wallet")
                .about("Manage local key pairs")
                .subcommand_required(true)
                .subcommand(Command::new("create").about("Generate a key pair and print its address"))
                .subcommand(Command::new("list").about("List the addresses in the local keystore")),
        )
        .subcommand(
            Command::new("chain")
                .about("Create and inspect the chain")
                .subcommand_required(true)
                .subcommand(
                    Command::new("init")
                        .about("Create a new chain; the genesis reward goes to ADDRESS")
                        .arg(positional("address", "Address that receives the genesis reward")),
                )
                .subcommand(Command::new("show").about("Print every block, newest first")),
        )
        .subcommand(
            Command::new("balance")
                .about("Show the balance of an address")
                .arg(positional("address", "Address to look up")),
        )
        .subcommand(
            Command::new("send")
                .about("Send coins; mines a block locally unless --node is given")
                .arg(positional("from", "Sender address (needs a key in the local keystore)"))
                .arg(positional("to", "Recipient address"))
                .arg(positional("amount", "Number of coins"))
                .arg(
                    Arg::new("node")
                        .long("node")
                        .value_name("HOST:PORT")
                        .help("Submit the transaction to a running node instead of mining it here"),
                ),
        )
        .subcommand(
            Command::new("utxo")
                .about("Maintain the unspent output index")
                .subcommand_required(true)
                .subcommand(Command::new("rebuild").about("Recompute the index from the chain")),
        )
        .subcommand(
            Command::new("node")
                .about("Run a network node")
                .subcommand_required(true)
                .subcommand(
                    Command::new("start")
                        .about("Listen on 127.0.0.1:PORT and talk to the seed node")
                        .arg(positional("port", "TCP port to listen on"))
                        .arg(
                            Arg::new("miner")
                                .long("miner")
                                .value_name("ADDRESS")
                                .help("Mine incoming transactions and pay rewards to ADDRESS"),
                        ),
                ),
        )
}

fn text<'a>(matches: &'a ArgMatches, name: &str) -> Result<&'a str> {
    matches
        .get_one::<String>(name)
        .map(|value| value.as_str())
        .ok_or_else(|| Error::from(format!("missing argument <{}>", name)))
}

fn run_wallet(sub: &ArgMatches) -> Result<()> {
    let store = Store::open()?;
    let keys = Keystore::open(&store)?;
    match sub.subcommand() {
        Some(("create", _)) => {
            let address = keys.create()?;
            println!("new address: {}", address);
            Ok(())
        }
        Some(("list", _)) => {
            println!("addresses:");
            for address in keys.addresses()? {
                println!("  {}", address);
            }
            Ok(())
        }
        _ => err("unknown wallet command"),
    }
}

fn run_chain(sub: &ArgMatches) -> Result<()> {
    let store = Store::open()?;
    match sub.subcommand() {
        Some(("init", args)) => {
            let address = text(args, "address")?;
            decode_address(address)?;
            let ledger = Ledger::init(&store, address)?;
            let utxo = UtxoSet::new(&store, ledger)?;
            let indexed = utxo.rebuild()?;
            println!("chain created; genesis reward sent to {}", address);
            println!("transactions with unspent outputs: {}", indexed);
            Ok(())
        }
        Some(("show", _)) => {
            let ledger = Ledger::load(&store)?;
            render::print_chain(&ledger);
            Ok(())
        }
        _ => err("unknown chain command"),
    }
}

fn run_balance(sub: &ArgMatches) -> Result<()> {
    let address = text(sub, "address")?;
    let owner = decode_address(address)?;
    let store = Store::open()?;
    let ledger = Ledger::load(&store)?;
    let utxo = UtxoSet::new(&store, ledger)?;
    println!("balance of {}: {}", address, utxo.balance(&owner)?);
    Ok(())
}

fn run_send(sub: &ArgMatches) -> Result<()> {
    let from = text(sub, "from")?;
    let to = text(sub, "to")?;
    let amount = text(sub, "amount")?
        .parse::<u64>()
        .map_err(|_| Error::from("amount must be a positive whole number"))?;

    let store = Store::open()?;
    let keys = Keystore::open(&store)?;
    let sender = keys
        .lookup(from)?
        .ok_or_else(|| Error::from(format!("no key for {} in the local keystore", from)))?;
    let ledger = Ledger::load(&store)?;
    let utxo = UtxoSet::new(&store, ledger.clone())?;

    let payment = Transaction::new_payment(&sender, to, amount, &utxo)?;
    match sub.get_one::<String>("node") {
        Some(node) => {
            net::submit_tx(node, &payment)?;
            println!("transaction {} submitted to {}", payment.id, node);
        }
        None => {
            // Mining locally: the sender collects the block reward.
            let reward = Transaction::coinbase(from, "mining reward")?;
            let block = ledger.mine_block(vec![reward, payment])?;
            utxo.apply_block(&block)?;
            println!("mined block #{} ({})", block.height(), block.hash());
            println!("sent {} from {} to {}", amount, from, to);
        }
    }
    Ok(())
}

fn run_utxo(sub: &ArgMatches) -> Result<()> {
    match sub.subcommand() {
        Some(("rebuild", _)) => {
            let store = Store::open()?;
            let ledger = Ledger::load(&store)?;
            let utxo = UtxoSet::new(&store, ledger)?;
            let indexed = utxo.rebuild()?;
            println!("index rebuilt: {} transactions with unspent outputs", indexed);
            Ok(())
        }
        _ => err("unknown utxo command"),
    }
}

fn run_node(sub: &ArgMatches) -> Result<()> {
    match sub.subcommand() {
        Some(("start", args)) => {
            let port = text(args, "port")?;
            let miner = args.get_one::<String>("miner").cloned();
            if let Some(address) = &miner {
                decode_address(address)?;
            }
            let store = Store::open()?;
            let ledger = Ledger::load(&store)?;
            let utxo = UtxoSet::new(&store, ledger.clone())?;
            let peer = Peer::new(format!("127.0.0.1:{}", port), miner, ledger, utxo);
            peer.serve()
        }
        _ => err("unknown node command"),
    }
}

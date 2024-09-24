use crate::wait;
use crate::unitctl::UnitCtl;
use crate::UnitctlError;
use crate::unitctl_error::ControlSocketErrorKind;
use crate::execute_cmd::send_and_deserialize;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use notify::event::DataChange::Content;
use notify::event::ModifyKind::Data;
use notify::EventKind::Modify;
use unit_client_rs::unit_client::UnitClient;
use crate::inputfile::InputFile;
use crate::output_format::OutputFormat::JsonPretty;


pub async fn cmd(
    cli: &UnitCtl,
    filename: &String
) -> Result<(), UnitctlError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // if there are control sock addresses passed in, and there are more than one, bail.
    if cli.control_socket_addresses.is_some() &&
        cli.control_socket_addresses.clone().unwrap().len() > 1 {
        return Err(UnitctlError::ControlSocketError{
            kind: ControlSocketErrorKind::General,
            message: "too many control sockets. specify at most one.".to_string(),
        });
    }

    // otherwise grab the client. This is done by either
    // 1. passing the single control socket into the wait for sockets, in which case it will try to
    //    communicate with that instance, or
    // 2. get all of the running instances, if there are more than 1, bail, otherwise select the
    //    single instance. If there are no running instances, bail as well.
    //
    // @TODO: if there are no running instances, launch one with the config file passed in and keep
    //        that one updated.
    let mut control_sockets = wait::wait_for_sockets(cli).await?;
    let client = UnitClient::new(control_sockets.pop().unwrap());

    println!("hello world from the watch command with filename {}", filename);

    let p = Path::new(filename);

    let res = watch(p, client).await;

    println!("got a result: {:?}", res);

    Ok(())
}

async fn watch<P: AsRef<Path>>(path: P, client: UnitClient) -> notify::Result<()> {
    println!("creating the channels");
    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    println!("created the watcher");
    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;

    let method = String::from("PUT");

    for res in rx {
        match res {
            Ok(event) => {
                match event.kind {
                    Modify(Data(Content)) => {
                        log::info!("we had a content change");

                        if event.paths.len() > 1 {
                            log::warn!("there were more than one paths that changed, this should not have happened: {:?}", event.paths);
                            continue
                        }

                        if event.paths.len() == 0 {
                            log::warn!("there were zero paths. Not even sure how this is possible");
                            continue
                        }

                        let path: &Path = event.paths[0].as_ref();


                        log::info!("does this execute?");

                        let f = Some(InputFile::from(path));

                        log::info!("sending and deserialising");
                        let _ = send_and_deserialize(
                            &client,
                            method.clone(),
                            f,
                            "/config",
                            &JsonPretty
                        ).await
                            .map_err(|e| {
                                log::error!("error happened on send and deserialize: {}", e);
                                // std::process::exit(e.exit_code());
                            });

                        log::info!("everything worked fine apparently?");
                    }
                    _ => {
                        log::info!("Not a content Change: {event:?}");

                        ()
                    }
                }
            },
            Err(error) => log::error!("Error: {error:?}"),
        }
    }

    Ok(())
}
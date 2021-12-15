use std::thread::JoinHandle;

use super::formats::shared;
use ps_core::{
    crossbeam_channel::{self, unbounded},
    Config, DatabaseLike, Importerlike, Message, MessageReceiver,
};

use super::formats::ImporterFormat;

use eyre::Result;

pub struct Importer<Format: ImporterFormat> {
    config: Config,
    format: Format,
}

impl<Format: ImporterFormat + 'static> Importer<Format> {
    pub fn new(config: Config, format: Format) -> Self {
        Self { config, format }
    }
}

impl<Format: ImporterFormat + 'static> Importerlike for Importer<Format> {
    fn import<Database: DatabaseLike + 'static>(
        self,
        database: Database,
    ) -> Result<(MessageReceiver, JoinHandle<Result<()>>)> {
        let Importer { format, .. } = self;
        let (sender, receiver) = unbounded();

        let config = self.config;
        let handle: JoinHandle<Result<()>> = std::thread::spawn(move || {
            let outer_sender = sender.clone();
            let processed = move || {
                let emails = format.emails(&config, sender.clone())?;
                let processed =
                    shared::database::into_database(&config, emails, sender.clone(), database)?;

                Ok(processed)
            };
            let result = processed();

            // Send the error away and map it to a crossbeam channel error
            match result {
                Ok(_) => Ok(()),
                Err(e) => match outer_sender.send(Message::Error(e)) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(eyre::Report::new(e)),
                },
            }
        });
        Ok((receiver, handle))
    }
}

// impl<T: Importerlike + Sized> Importerlike for Box<T> {
//     fn import(self) -> Result<(MessageReceiver, JoinHandle<Result<()>>)> {
//         (*self).import()
//     }
// }

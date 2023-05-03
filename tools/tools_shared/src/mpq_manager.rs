use tokio::sync::{
    mpsc::{self, Sender},
    oneshot,
};

pub struct MPQManager {
    sender: Sender<GetFileDataCmd>,
}

type FileDataSender<T> = oneshot::Sender<Result<T, std::io::Error>>;

#[derive(Debug)]
pub struct GetFileDataCmd {
    file_path: String,
    sender: FileDataSender<Option<Vec<u8>>>,
}

impl MPQManager {
    pub fn new(client_data_dir: &str) -> Result<MPQManager, std::io::Error> {
        let (tx, mut rx) = mpsc::channel::<GetFileDataCmd>(32);
        let mpq_context = crate::open_mpqs(client_data_dir)?;

        let _ = tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                let result = crate::get_file_data(cmd.file_path, &mpq_context);

                let _ = cmd.sender.send(result);
            }
        });

        Ok(Self { sender: tx })
    }

    pub async fn get_file_data(
        &self,
        file_path: String,
    ) -> oneshot::Receiver<Result<Option<Vec<u8>>, std::io::Error>> {
        let (sender, receiver) = oneshot::channel();
        let cmd = GetFileDataCmd { file_path, sender };

        self.sender.send(cmd).await.unwrap();

        receiver
    }
}


#[derive(Debug)]
pub enum DataType {
    FinalizedBlock,
    NewHead,
}

#[derive(Debug)]
pub struct Data {
    data_type: DataType
}

impl Data {
    pub fn new(data_type: DataType) -> Self {

        Self { data_type }
    }
}

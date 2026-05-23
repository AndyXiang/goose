use super::*;
use uuid::Uuid;

pub struct DataHandler<'a> {
    db: &'a DataBase,
}

impl<'a> DataHandler<'a> {
    pub fn new(db: &'a DataBase) -> Self {
        Self { db }
    }

    pub fn get_bar(
        &self,
        id: Uuid,
        start: TimeStamp,
        end: TimeStamp,
    ) -> Result<Vec<(TimeStamp, Bar)>> {
        self.db
            .get_bar(&id.to_string(), &start.to_string(), &end.to_string())?
            .into_iter()
            .map(|(ts, bar)| Ok((ts.parse()?, bar.try_into()?)))
            .collect()
    }

    pub fn register_entity(&self, entity: impl Entity) -> Result<usize> {
        self.db.register_entity(entity)
    }

    pub fn execute(&self, sql: &str) -> Result<usize> {
        self.db.execute(sql)
    }
}

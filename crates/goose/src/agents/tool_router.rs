use tantivy::schema::*;
use tantivy::{Index, ReloadPolicy, Term};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::TantivyDocument;
use etcetera::{choose_app_strategy, AppStrategy};
use std::path::PathBuf;
use uuid::Uuid;

pub struct ToolRouter {
    index: Index,
    reader: tantivy::IndexReader,
    schema: Schema,
    id_field: Field,
    description_field: Field,
    _index_dir: PathBuf,  // Keep the path alive for the lifetime of the index
}

impl ToolRouter {
    fn build_schema() -> Schema {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT);
        schema_builder.build()
    }

    fn write_documents(index: &Index, schema: &Schema, tool_descriptions: &[(&str, &str)]) -> tantivy::Result<()> {
        let mut writer = index.writer::<TantivyDocument>(50_000_000)?;
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();

        for (title, body) in tool_descriptions {
            let mut doc = TantivyDocument::default();
            doc.add_text(title_field, title);
            doc.add_text(body_field, body);
            writer.add_document(doc)?;
        }
        writer.commit()?;
        Ok(())
    }

    pub fn new(tool_descriptions: Vec<(&str, &str)>) -> tantivy::Result<Self> {
        // Create schema
        let schema = Self::build_schema();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        
        // Create persistent index directory in goose config
        // - macOS/Linux: ~/.config/goose/toolrouter/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\toolrouter\
        let mut index_dir = choose_app_strategy(crate::config::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir("toolrouter"))
            .unwrap_or_else(|_| PathBuf::from(".config/goose/toolrouter"));

        // Append a random UUID to create a unique subdirectory
        let unique_dir = Uuid::new_v4().to_string();
        index_dir.push(unique_dir);

        // Ensure the directory exists
        std::fs::create_dir_all(&index_dir).expect("Failed to create toolrouter directory");
        
        // Create index in the persistent directory
        let index = Index::create_in_dir(&index_dir, schema.clone())?;
        
        // Write documents to index
        Self::write_documents(&index, &schema, &tool_descriptions)?;
        
        // Create reader
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            schema,
            id_field: title_field,
            description_field: body_field,
            _index_dir: index_dir,
        })
    }

    pub fn remove_document(&self, title: &str) -> tantivy::Result<()> {
        let mut writer = self.index.writer::<TantivyDocument>(50_000_000)?;
        let title_field = self.schema.get_field("title").unwrap();

        // Create a term for the title we want to delete
        let term = Term::from_field_text(title_field, title);

        // Delete all documents matching this term
        writer.delete_term(term);

        // Commit the changes
        writer.commit()?;

        // Reload the reader to reflect the changes
        self.reader.reload()?;
        
        Ok(())
    }

    pub fn match_tools(&self, user_query: &str, top_k: usize) -> tantivy::Result<Vec<String>> {
        self.reader.reload()?; // Refresh index state
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.description_field]);
        let query = query_parser.parse_query(user_query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(top_k))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(id_val) = retrieved_doc.get_first(self.id_field) {
                if let Some(id_text) = id_val.as_str() {
                    results.push(id_text.to_string());
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_router_basic_operations() -> tantivy::Result<()> {
        // Create test data
        let tool_descriptions = vec![
            ("Tool 1", "This is the first tool description"),
            ("Tool 2", "This is the second tool description"),
            ("Tool 3", "This is the third tool description"),
        ];

        // Create router
        let router = ToolRouter::new(tool_descriptions)?;

        // Test searching
        let results = router.match_tools("first tool", 1)?;
        println!("Results: {:?}", results);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "Tool 1");

        // Test removing a document
        // TODO: implement remove document so documents can be removed from the index in time
        // router.remove_document("Tool 1")?;

        // // Verify the document was removed
        // let results_after_removal = router.match_tools("first tool", 1)?;
        // assert_eq!(results_after_removal.len(), 0);

        // Test searching for remaining documents
        let results = router.match_tools("second tool", 1)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "Tool 2");

        Ok(())
    }

    #[test]
    fn test_tool_router_empty_search() -> tantivy::Result<()> {
        let tool_descriptions = vec![
            ("Tool 1", "This is the first tool description"),
            ("Tool 2", "This is the second tool description"),
        ];

        let router = ToolRouter::new(tool_descriptions)?;

        // Test searching with non-matching query
        let results = router.match_tools("nonexistent", 1)?;
        assert_eq!(results.len(), 0);

        Ok(())
    }

    #[test]
    fn test_tool_router_remove_nonexistent() -> tantivy::Result<()> {
        let tool_descriptions = vec![
            ("Tool 1", "This is the first tool description"),
        ];

        let router = ToolRouter::new(tool_descriptions)?;

        // Test removing a non-existent document
        router.remove_document("Nonexistent Tool")?;

        // Verify the original document still exists
        let results = router.match_tools("first tool", 1)?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], "Tool 1");

        Ok(())
    }
}
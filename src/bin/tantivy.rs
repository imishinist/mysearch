use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Schema, STORED, TEXT},
    DocAddress, Index, Score,
};

fn main() {
    let mut schema_builder = Schema::builder();
    let title = schema_builder.add_text_field("title", TEXT | STORED);
    let body = schema_builder.add_text_field("body", TEXT);
    let schema = schema_builder.build();

    let index = Index::create_in_ram(schema.clone());

    let mut index_writer = index.writer(100_000_000).unwrap();

    index_writer.add_document(doc!(
        title => "THE STORY OF MANKIND",
        body => "WHEN I was twelve or thirteen years old, an uncle of mine who gave me my love for books and pictures promised to take me upon a memorable expedition.",
    ));
    index_writer.commit().unwrap();

    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let query_parser = QueryParser::for_index(&index, vec![title, body]);

    let query = query_parser.parse_query("thirteen years").unwrap();
    let top_docs: Vec<(Score, DocAddress)> =
        searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address).unwrap();
        println!("{}", schema.to_json(&retrieved_doc));
    }
}

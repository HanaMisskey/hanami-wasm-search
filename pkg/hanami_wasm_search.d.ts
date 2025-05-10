/* tslint:disable */
/* eslint-disable */
export class Index {
  free(): void;
  constructor();
  static newWithParams(k1: number, b: number): Index;
  set_params(k1: number, b: number): void;
  add_documents(json: string): void;
  searchWithMode(query_json: string, _mode: string, limit?: number | null): any;
  searchWithModeNoLimit(query_json: string, mode: string): any;
  searchWithLimit(query_json: string, mode: string, limit: number): any;
  dump(): Uint8Array;
  static load(bytes: Uint8Array): Index;
  removeDocument(doc_id: string): boolean;
  addDocument(name: string, aliases_json: string): void;
  updateDocument(doc_id: string, aliases_json: string): boolean;
  replaceAllDocuments(json: string): void;
  clearIndex(): void;
}

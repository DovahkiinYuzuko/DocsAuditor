# Development Rules for AI Agents

- **Specification Updates**:
    - **Workflow Priority**: You must first inspect or update the corresponding variable/function specification document before modifying any source code to optimize token usage.
    - **File Management & Naming**:
        - All specification documents must be managed under `docs\variables'n'functions` and created on a per-source-file basis.
        - Before creating or updating any document, inspect the root folder using `ls -R` to confirm if `docs\variables'n'functions` exists. If it does not, create the folder first.
        - File names must follow the convention `[Language]filename.md` (e.g., `[TypeScript]UserService.md` or `[Python]auth.md`).
        - Do not include line numbers in the document.
    - **Formatting & Heading Rules**:
        - Whenever variables or functions are created, modified, or referenced, you must always enclose their names in backticks (` `).
        - Within the specification body, all variables and functions must be documented under Heading 3 (`###`) or lower headings (e.g., `####`).
        - When a backtick-enclosed name is used in a heading, **no text or characters are allowed to follow immediately after the closing backtick**.
            - *Allowed Examples*: `### (Function) `Sum``, `#### `Substract``
            - *Disallowed Examples*: `### `Sum`(Function)`
  **Every specification document must strictly begin with a YAML front matter** formatted as follows to maximize searchability, index efficiency, and token-saving during AI filtering:
  ```yaml
  ---
  source_file: "relative/path/to/source_file"
  language: "LanguageName"
  description: "A concise, one-sentence summary of this file's role and responsibilities."
  tags:
    - "@Category"
    - "@ComponentNameOrFeature"
  exports:
    - NamedElement1
    - NamedElement2
  imports:
    - "relative/path/to/imported_file_1"
    - "relative/path/to/imported_file_2"
  ---
  ```
- **Tag Management & Governance (`tag-index.md`)**: Any tag used in the YAML front matter, regardless of the document type, must strictly adhere to the predefined valid tags listed in `docs\tag-index.md` (all tags must start with `@`).
    - **Prohibition of Arbitrary Tags**: The agent must never create or apply arbitrary or novel tags that are not already defined in the index.
    - **User Consultation**: If the existing tags are insufficient or a new tag needs to be added, you must always consult the user and obtain explicit permission before modifying any source code, specifications, or the index file.
    - **Index Synchronization**: Whenever a tag is added, removed, or modified in any document, you must immediately update the corresponding tag's usage list in `docs\tag-index.md` to keep the index completely synchronized and up-to-date.
    - **`tag-index.md` Format Standard**: The `docs\tag-index.md` file must strictly follow the lightweight structure below:
      ```markdown
      # Tag Index

      ## Tags

      ### `@[tag]`
      * **Description:** [Brief explanation of the tag's purpose and semantic context]
      * **Usage:**
        * [relative/path/to/specification_1.md]
        * [relative/path/to/specification_2.md]
      ```
- **Definition Updates**: Reflect any changes to types, arguments, and return values in the specification first.
- **Dependency Tracking**:
    - **Dependency Mapping**: You must explicitly map all dependencies using both the **YAML front matter (`imports`)** for fast AI machine-parsing, and **Mermaid diagrams** within the specification document body for visual clarity.
    - **Impact Scope**: Identify and document which existing parts of the codebase are affected by this change based on the defined imports.
- **Consistency Assurance**: Ensure the documentation (both YAML metadata and Markdown body) is completely synchronized with the generated code in the same turn, keeping it aligned with the automated audit tool's feedback.
- **Audit Report Verification**: Before declaring any task as complete, you must check the project root for the existence of `variables_functions_audit_report.md`. If this file exists, you must read it immediately, correct all reported inconsistencies or dead code in the source code or specifications, and ensure the report is resolved (deleted by the system) before finishing.

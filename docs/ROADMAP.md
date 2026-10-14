# Next work, in release priority order

1. Execute clean Windows installer/file association/sign/print/uninstall acceptance;
   test native Credential Manager and the GDI printing path. Fix any runtime issues.
2. Isolate untrusted parsing/rendering in a process, enforce memory/CPU limits,
   fuzz parser boundaries, update/review PDFium, audit the signer and dependencies.
3. Finish GUI acceptance coverage: selection/clipboard, all forms and annotations,
   native dialogs, visual signing, local-identity setup, restart/reuse, saving failures.
4. Add embedded Unicode form/free-text appearance fonts, proper multiline wrapping,
   third-party annotation editing, internal/external links and named destinations.
5. Add existing signature-field signing and permission-aware DocMDP/FieldMDP handling.
   Keep conservative blocks until independently validated; never infer permission
   from the fact that an incremental update preserves bytes.
6. Add identity import/backup/rollover with explicit protection and recovery UX;
   adapt the signer for non-exportable CNG/Keychain provider callbacks.
7. Finish signature cropping, saved-appearance rename/default, date/checkmark tools,
   rotation-accurate previews, and annotation resizing handles.
8. Native macOS printing, platform acceptance, accessibility audits, performance
   corpus/benchmarks and cancellation. Validate external readers individually.
9. Only after every mandatory v1 gate passes: tag a v1 release and publish signed
   installers. Current artifacts are development builds without code signing.

Future v2: OCR, page organization, conversion, real redaction, content editing,
comparison, trusted identities, optional TSA/revocation/remote signing. None is
implemented as a misleading placeholder in the v1 toolbar.

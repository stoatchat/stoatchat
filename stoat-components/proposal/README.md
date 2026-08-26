# Stoat Interactive Components - Proposal

This proposal adds support for interactive components (buttons and select menus) for bots on Stoat.

## Proposal Structure

```
proposal/
├── README.md
├── RFC_INTERACTIVE_COMPONENTS.md
└── docs/
    ├── interactive-components.md
    └── implementation-guide.md
```

## Summary

### What is it

Interactive components are UI elements that allow users to interact with bot messages:

- **Buttons**: Actions with different styles (primary, success, danger, link)
- **Select Menus**: Dropdowns for option selection

### Why

- Bots need better ways to interact with users
- Improves user experience
- Compatible with industry standards (Discord, etc.)

### How

1. Add `components` to `DataMessageSend` schema
2. Create `interaction_create` events
3. Create interaction response endpoint

## Files

### [Full RFC](RFC_INTERACTIVE_COMPONENTS.md)

Complete technical documentation including:
- API schemas
- Usage examples
- Events and responses
- Limits

### [Developer Documentation](docs/interactive-components.md)

Documentation that can be added to the Stoat docs site:
- Component types
- Properties
- Usage examples
- Limits

### [Implementation Guide](docs/implementation-guide.md)

Technical guide for implementing in the Rust backend:
- Data structures
- Validation
- Events
- Tests

## How to Contribute

1. Fork the Stoat repository
2. Implement changes following the guide
3. Add tests
4. Submit a Pull Request

## Status

- [x] Documentation created
- [x] RFC ready
- [x] Implementation guide
- [ ] Backend implementation
- [ ] Tests
- [ ] Official documentation

## Contact

- GitHub: https://github.com/stoatchat/stoat
- Discord: https://stt.gg/Testers

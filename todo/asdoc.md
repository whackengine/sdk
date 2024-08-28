# ASDoc

## Sections and assets

Additional assets and Markdown sections may be described in the Flex package's `asdoc.json` file, used when rendering the ASDoc documentation for a Flex package.

```json
{
    "base-path": "docs",
    "assets": [
        {
            "path": "image.png"
        }
    ],
    "home": {
        "title": "My Package's Home",
        "path": "index.md"
    },
    "sections": [
        {
            "title": "Foo",
            "path": "foo.md",
            "children": [
                {
                    "title": "Qux",
                    "path": "qux.md"
                }
            ]
        }
    ]
}
```

## Manual documentation

### Object

Manually document public::constructor, prototype::hasOwnProperty(), prototype::toLocaleString(), prototype::toString() and prototype::valueOf(), as these do not appear as fixtures.
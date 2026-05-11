# Testing NADI Md

Here is a simple test for nadi md file. Nadi md files are markdown files with nadi codes in them that can be run and outputs shown.

Code in code blocks will run if provided with `run` keyword, the results will follow.

```task run
net.load_str("a -> b")
nodes.NAME
```

Every run is done in the same context.
```task run
nodes.NAME
```

If you want to have a new session, use the `new` option to clear the context.

```task run new
nodes.NAME
```

If an error occured it will also show that, but it will not halt the whole rendering process, so be careful about that. Future versions will add a flat to halt the process.

```task run
somefunc(12, 90, true)
```

#set par(spacing:1em, leading: .4em, justify: true)
#set raw(syntaxes: "typst/task.sublime-syntax")
#set raw(syntaxes: "typst/signature.sublime-syntax")
#set raw(syntaxes: "typst/stp.sublime-syntax")

#show raw: set block(fill: luma(230), inset: 3pt, radius: 2pt, width: 100%)
#show heading: it => [
    #block(above: 2em, below: 2em, it)
]


= Testing NADI Md



Here is a simple test for nadi md file. Nadi md files are markdown files with nadi codes in them that can be run and outputs shown.

Code in code blocks will run if provided with `run` keyword, the results will follow.


``````task
net.load_str("a -> b")
nodes.NAME
``````
Results:
``````output
["b", "a"]
``````



Every run is done in the same context.


``````task
nodes.NAME
``````
Results:
``````output
["b", "a"]
``````



If you want to have a new session, use the `new` option to clear the context.


``````task
nodes.NAME
``````
Results:
``````output
[]
``````



If an error occured it will also show that, but it will not halt the whole rendering process, so be careful about that. Future versions will add a flat to halt the process.


``````task
somefunc(12, 90, true)
``````
\*Error\*:
``````error
FunctionNotFoundError  at Line 1 Column 1: env function named "somefunc" not found
``````

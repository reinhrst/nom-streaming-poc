This repo shows a PoC of a streaming parser in [nom][1] (Rust), as requested in [this discussion][2].

You may just skip to the code straight away, however I do think that a bit of discussion is very enlightening.
It also explains why the PoC is a 100+ line program, rather than 3 lines.

Please don't take this to be any kind of official, or even officially endorsed, documentation; I ran into the GitHub issue mentioned above (while wanting to create my own streaming parser) , and realised that it would be a good exercise to write this PoC.
I should also mention that this is the first Rust program that I ever made (or at least, the first not printing "Hello World"), so I may very well have included a bunch of un-rustic stuff in there.
However, looking at the output, it seems to be working....

## What we mean by streaming parsing

It makes sense to quickly discuss what we actually mean by streaming parsing.
"Complete" parsing (which is the other, non-streaming, method that nom supports) takes input-(byte or string)-data and returns some sort of higher level objects, based on your code.
It needs to have all data in memory up-font, and returns the full result in the end.

There are two reasons why this may not work in some situations:

- The input-data is (or can possibly be) large, and we don't want to have it all in memory at the same time -- depending on how you parse, this may also mean that the output-objects will be large.[^large]
- The input data may only arrive slowly (i.e. you receive the data over the network) and you may want to start parsing the things you already have received (and do something with the resulting objects), before all data has arrived.

Generally, in both cases, we need a parser that streams on both sides: we both want to stream in the data (bytes or string), and stream out the parsed objects on the other end.[^both-ends]

Where examples are given in this document, I will keep the input strings short for readability, just imagine them to be huge in your mind :).

## How does streaming work in nom

(This is the part where I say lots of things that only _expect_ to be true....)

nom exposes two API's for parsing: `complete` (in `bytes::complete` and `character::complete`) and `streaming` (in `bytes::streaming` and `character::streaming`).
These API's differ from each other in some small ways, specifically in that the `streaming` API will return `Err::Incomplete(Needed::new(M))`  if additional data either is needed to parse, or could lead to a different parsing result.[^different-parsing-result]

The idea is that the application code will detect the `Err::Incomplete` error and resolve it.
Since nom's API works by functions getting the input-data as a (read-only) parameter, and returning the parsed object and the not-yet-processed part of the input data, recovering from an `Err::Incomplete` error is relatively easy.
At the place where the application catches this error, the original input-data is still available, so the application can just keep the old input data, append more data and try again (nom's parser doesn't have an internal state, so no need to worry about that; you just try the same parsing, just with a bit more data).[^no-side-effects]

Since parsing functions actually return the bytes left-over, adding data (streaming-in data) can happen not only at the top-level of your program, but at any spot _where data is guaranteed to be consumed_.[^guaranteed-to-be-consumed]

## Solution for streaming in this PoC

In this PoC I chose to implement streaming using `Iterator`s.
On the input-side, there is a `FileIterator`, which produces additional bytes on every call to `next()`.
On the output side, there is an Iterator producing output objects (`ComplexStructure`s in my case).
The whole system is lazy: only when a new output object is requested from the iterator, it gets parsed.
If parsing fails on `Err::Incomplete`, more data gets requested from the `FileIterator`, until parsing succeeds.

I'm sure there are other ways to implement the streaming-concept (and the POC can save one data-copy operation if the input is streaming directly from a file, rather than through an iterator).

## Differences between the streaming and complete parser code

Whereas writing a `complete` parser is relatively simple (you keep combining small parsers, until you have one large parser that parses the whole file and returns the result, usually some "tree-like" combination of Structs and Vectors of Structs), this approach needs refinement in case of `streaming`; most specifically, determine where (in the tree-like thing) the streaming has to take place.

For instance, some file may contain a lot of items at the top-level (e.g. a log-file contains lots of lines; each line may get parsed further, but the top-level is lines, or the parsed structure is `Vec<String>`).

The `complete` parser may look something like:

```rust
let (input, output) = multi::many0(parse_line(input))
```

The `streaming` version will need to do the `many0` itself.
That actually means having an Iterator that internally keeps track of the `input` variable

```rust
struct LinesIterator {
    input: InputIter + InputTake + InputLength,
}

impl Iterator for LinesIterator {
    type Item = String;

    fn next(&mut self) -> Option<String> {
    loop {
        match parse_line(self.input) {
            Err(Err::Incomplete) => {self.input = get_more_data(self.input)}
            Ok(input, output) => {
                self.input = input;
                Some(output)
            }
        }
    }
}
```

So whenever a next line is requested from the `LinesIterator`, `parse_line()` is called.
If it has enough data, a new line is returned.
If not, more data is requested and the whole thing is tried again.

One thing that we don't have in this code, is a way to gracefully end parsing (if no more data is available, then the result should be 


TODO!: https://github.com/rust-bakery/nom/blob/main/doc/custom_input_types.md



Two things are missing in the code above:

- How to deal with streaming of the input
- How to gracefully have the `Iterator` return `None` when end-of-input has been reached.


## End of file / End of stream problem

We discussed before that there are certain cases where the `streaming` parser asks for more data, while the `complete` parser would succeed; for instance if parsing a number, and you have `123`.
Generally when using the streaming parser, it's good that the `Err::Incomplete` is thrown in this case, and the streaming parser waits to see if the next data is a digit or not.
However, once we have reached the end of the data (End of File / End of Stream), we would like the parser to behave like a `complete` parser.

As far as I know, there is no easy way to force this, which means that either one has to implement the whole parser as `complete` parser as well (and switch to that one once the end of the data was reached), or come up with some other smart plan (if you know that the last thing you parse is a number, you could add a <space> to the data at end-of-data, and then assert that you only have a space left).



## Limitations / anti-goals for PoC

The PoC is showing how a streaming parser based on nom can be implemented.
The PoC parser copies all data around a couple of times.
If raw speed and extreme datasets are important for you, you should probably optimise the system a bit further; I would argue that in 99% of cases, with today's `memcpy()` speeds, you will not notice much slowdown for this.




TODO: https://www.sublimetext.com/blog/articles/use-mmap-with-care
https://users.rust-lang.org/t/how-unsafe-is-mmap/19635
https://github.com/rust-bakery/nom/blob/main/doc/custom_input_types.md



[^large]: Note that it's up to the app developer to decide what "large" is in this context.
Obviously when your file in 10GB and your computer has 8GB RAM, it will not fit; however also if you never deal with files of more than 4GB, you probably don't want your program to need 4GB of RAM, when it could also run (streaming) in 100MB.

[^both-ends]: You could come up with situations where this is not technically necessary; for instance, if you had a parser that (for some reason) skips most of the input stream as not-needed, and only returns 5 small result-objects, even if the input is 100GB (think of something like `grep` but then in a parser), you _could_ build something that only streams on the input end. I think you could some up with some other exceptional circumstances where only input-side streaming is necessary, but I think these are all very contrived exceptions.

One situation where one could consider only having the output-side streaming (and the input side not), is if one would use a memory-mapped file as input; although maybe this should be considered cheating, since this just means that the OS does the input-streaming for you.

[^different-parsing-result]: Examples of when parsing would be successful, but more data could lead to a different parsing result:

- A parse-step to read a number (`[0-9]*`), if it sees input `123` the `complete` parser could return `123`, however the streaming parser does not know if the next character is not `4`.
- A situation where parsing has multiple branches (branch 1 matches: `ABCD 1234`, branch 2 matches `ABCD`, when data is `ABCD 12`; `complete` parser would return branch 2 (with ` 12` left over) whereas `streaming` parser should wait to see what the next characters are.

[^no-side-effects]: Note that this is only true if your parsing functions have no (meaningful) [side effects][3]; I don't expect that parsing functions with side effects are ever useful or advisable (they will also not work nice with branches). With _meaningful_ (in meaningful side effects), I mean anything else than writing something to stdout / a log file, which a human looking at the output can just ignore.

[^guaranteed-to-be-consumed]: When new data is streamed in, this data needs to travel up the tree to (eventually) top level. This works by functions returning the data with their (parsed) output.
If parsing fails (e.g. the parser is looking for a number and finds `a`), an error is returned.
If this is the only branch, this is not a problem (since all of parsing fails), but if the parser will try another branch in case of a problem, the streamed-in data is lost.

Examples of where other branches are tried:

- `nom::branches::alt` and `nom::branches::permutations` (Obviously these create branches)
- `nom::multi::*` for those functions that repeat until they fail (e.g. `many0` will repeat the inner parser until it gives an error). The data in the last (failing) run of the parser is not consumed, rather if it fails, another branch (where the `many0` ends on the previous element) is tried.
- Any point where you may have manual code that catches a parsing error and tries another branch.

Note that I'm not sure the above list of exhaustive; use common sense to determine to determine where the branches are.




[1]: https://github.com/rust-bakery/nom
[2]: https://github.com/rust-bakery/nom/issues/1160#issuecomment-721009263
[3]: https://en.wikipedia.org/wiki/Side_effect_(computer_science)

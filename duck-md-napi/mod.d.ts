export interface TocItem {
    title: string;
    url: string;
    items: TocItem[];
}
export interface Metadata {
    readingTime: number;
    wordCount: number;
}
export interface CompileOutput {
    body: string;
    content: string;
    html: string;
    excerpt: string;
    metadata: Metadata;
    toc: TocItem[];
    frontmatter: unknown;
    frontmatterRaw: string;
    imports: string[];
    exports: string[];
}
export type SchemaKind = 'string' | 'number' | 'boolean' | 'array' | 'object' | 'record' | 'tuple' | 'intersection' | 'enum' | 'literal' | 'union' | 'discriminatedUnion' | 'optional' | 'nullable' | 'default' | 'transform' | 'refine' | 'superRefine' | 'coerce.string' | 'coerce.number' | 'coerce.boolean' | 'coerce.date' | 'raw' | 'markdown' | 'mdx' | 'toc' | 'metadata' | 'excerpt' | 'path' | 'slug' | 'unique' | 'isodate' | 'file' | 'image';
export interface SchemaDescriptor {
    kind: SchemaKind;
    [field: string]: unknown;
}
export interface CollectionConfig<S = unknown> {
    name?: string;
    pattern: string | string[];
    baseDir?: string;
    single?: boolean;
    schema?: SchemaBuilder<S> | SchemaDescriptor;
}
export interface OutputOptions {
    data?: string;
    assets?: string;
    base?: string;
    name?: string;
    clean?: boolean;
    format?: 'esm' | 'cjs';
}
export interface MarkdownOptions {
    gfm?: boolean;
    removeComments?: boolean;
    copyLinkedFiles?: boolean;
    remarkPlugins?: unknown[];
    rehypePlugins?: unknown[];
}
export interface MdxOptions extends MarkdownOptions {
    outputFormat?: 'function-body' | 'module';
    minify?: boolean;
}
export interface UserConfig {
    root?: string;
    strict?: boolean;
    output?: OutputOptions;
    collections: Record<string, CollectionConfig>;
    loaders?: unknown[];
    markdown?: MarkdownOptions;
    mdx?: MdxOptions;
    prepare?: (data: Record<string, unknown[]>, ctx: {
        config: UserConfig;
    }) => unknown;
    complete?: (data: Record<string, unknown[]>, ctx: {
        config: UserConfig;
    }) => unknown;
}
export interface BuildCollectionReport {
    name: string;
    records: number;
    outputPath: string;
}
export interface BuildErrorReport {
    file: string;
    message: string;
}
export interface BuildReport {
    collections: BuildCollectionReport[];
    errors: BuildErrorReport[];
}
export declare class SchemaBuilder<_T = unknown> {
    [k: string]: unknown;
    constructor(descriptor: SchemaDescriptor);
    toJSON(): SchemaDescriptor;
    optional(): SchemaBuilder;
    nullable(): SchemaBuilder;
    default(value: unknown): SchemaBuilder;
    min(n: number): SchemaBuilder;
    max(n: number): SchemaBuilder;
    length(n: number): SchemaBuilder;
    regex(p: string): SchemaBuilder;
    int(): SchemaBuilder;
    by(bucket: string): SchemaBuilder;
    reserved(list: string[]): SchemaBuilder;
    passthrough(): SchemaBuilder;
    transform(fn: (v: unknown) => unknown): SchemaBuilder;
    refine(fn: (v: unknown) => boolean, message?: string): SchemaBuilder;
}
export interface SBuilders {
    string(): SchemaBuilder<string>;
    number(): SchemaBuilder<number>;
    boolean(): SchemaBuilder<boolean>;
    array<I>(item: SchemaBuilder<I>): SchemaBuilder<I[]>;
    object<S extends Record<string, SchemaBuilder>>(fields: S): SchemaBuilder;
    record<V>(value: SchemaBuilder<V>): SchemaBuilder<Record<string, V>>;
    tuple(items: SchemaBuilder[]): SchemaBuilder<unknown[]>;
    intersection<A, B>(a: SchemaBuilder<A>, b: SchemaBuilder<B>): SchemaBuilder<A & B>;
    enum<T>(variants: T[]): SchemaBuilder<T>;
    literal<T>(value: T): SchemaBuilder<T>;
    union<T>(variants: SchemaBuilder<T>[]): SchemaBuilder<T>;
    discriminatedUnion<T>(discriminator: string, variants: SchemaBuilder<T>[]): SchemaBuilder<T>;
    coerce: {
        string(): SchemaBuilder<string>;
        number(): SchemaBuilder<number>;
        boolean(): SchemaBuilder<boolean>;
        date(): SchemaBuilder<string>;
    };
    raw(): SchemaBuilder<string>;
    markdown(): SchemaBuilder<string>;
    mdx(): SchemaBuilder<string>;
    toc(): SchemaBuilder<TocItem[]>;
    metadata(): SchemaBuilder<Metadata>;
    excerpt(opts?: {
        length?: number;
    }): SchemaBuilder<string>;
    path(opts?: {
        removeIndex?: boolean;
    }): SchemaBuilder<string>;
    slug(bucket?: string, reserved?: string[]): SchemaBuilder<string>;
    unique(bucket?: string): SchemaBuilder<string>;
    isodate(): SchemaBuilder<string>;
    file(opts?: {
        allowNonRelativePath?: boolean;
    }): SchemaBuilder<string>;
    image(opts?: {
        absoluteRoot?: string;
    }): SchemaBuilder<{
        src: string;
        width: number;
        height: number;
    }>;
}
export declare const s: SBuilders;
export declare const defineConfig: (config: UserConfig) => UserConfig;
export declare const defineCollection: <S>(c: CollectionConfig<S>) => CollectionConfig<S>;
export declare const defineLoader: <L>(l: L) => L;
export declare const defineSchema: <S>(sch: S) => S;
export interface CustomLoader<T = unknown> {
    test: RegExp | string;
    load: (file: {
        path: string;
        value: string;
    }) => T | Promise<T>;
}
export declare function applyLoaders<T>(loaders: CustomLoader<T>[] | undefined, filePath: string, content: string): Promise<T | null>;
export declare function compile(source: string): CompileOutput;
export declare function compileMany(sources: string[]): CompileOutput[];
export declare function build(input: UserConfig): Promise<BuildReport>;
declare const _default: {
    compile: typeof compile;
    compileMany: typeof compileMany;
    build: typeof build;
    defineConfig: (config: UserConfig) => UserConfig;
    defineCollection: <S>(c: CollectionConfig<S>) => CollectionConfig<S>;
    defineLoader: <L>(l: L) => L;
    defineSchema: <S>(sch: S) => S;
    applyLoaders: typeof applyLoaders;
    s: SBuilders;
    SchemaBuilder: typeof SchemaBuilder;
};
export default _default;

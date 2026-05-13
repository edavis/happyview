import { docs, blog } from 'collections/server';
import { loader } from 'fumadocs-core/source';

export const source = loader(docs.toFumadocsSource(), {
  baseUrl: '/',
});

export const blogSource = loader(blog.toFumadocsSource(), {
  baseUrl: '/blog',
});

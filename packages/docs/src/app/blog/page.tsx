import { blogSource } from '@/lib/source';
import { VaporwaveGrid } from '@/components/vaporwave-grid';
import Image from 'next/image';
import Link from 'next/link';

export const metadata = {
  title: 'Blog',
  description: 'Project updates, release notes, and feature announcements.',
};

export default function BlogIndex() {
  const posts = blogSource
    .getPages()
    .sort((a, b) => b.data.date.getTime() - a.data.date.getTime());

  return (
    <div className="mx-auto max-w-3xl px-6 py-16">
      <h1 className="text-4xl font-bold mb-2">Blog</h1>
      <p className="text-lg mb-12" style={{ color: 'rgb(var(--color-fg-muted))' }}>
        Project updates, release notes, and feature announcements.
      </p>
      <div className="flex flex-col gap-6">
        {posts.map((post) => (
          <Link
            key={post.url}
            href={post.url}
            className="group block rounded-xl p-6 transition-all duration-200"
            style={{
              backgroundColor: 'rgb(var(--color-surface))',
              borderWidth: '1px',
              borderColor: 'rgb(var(--color-border))',
            }}
          >
            <h2
              className="text-xl font-semibold mb-2 transition-colors duration-200"
              style={{ color: 'rgb(var(--color-fg))' }}
            >
              {post.data.title}
            </h2>
            {post.data.description && (
              <p className="mb-4 text-sm" style={{ color: 'rgb(var(--color-fg-muted))' }}>
                {post.data.description}
              </p>
            )}
            <div className="flex flex-wrap items-center gap-4">
              <div className="flex items-center gap-2">
                <Image
                  src={post.data.author.avatar}
                  alt={post.data.author.name}
                  width={20}
                  height={20}
                  className="rounded-full"
                />
                <span className="text-xs" style={{ color: 'rgb(var(--color-fg-muted))' }}>
                  {post.data.author.name}
                </span>
              </div>
              <time
                dateTime={post.data.date.toISOString()}
                className="text-xs"
                style={{ color: 'rgb(var(--color-fg-muted))' }}
              >
                {post.data.date.toLocaleDateString('en-US', {
                  year: 'numeric',
                  month: 'long',
                  day: 'numeric',
                })}
              </time>
              {post.data.tags && post.data.tags.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {post.data.tags.map((tag) => (
                    <span
                      key={tag}
                      className="rounded-full px-2 py-0.5 text-xs font-medium"
                      style={{
                        backgroundColor: 'rgb(var(--color-magenta) / 0.12)',
                        color: 'rgb(var(--color-magenta))',
                      }}
                    >
                      {tag}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </Link>
        ))}
      </div>
      <VaporwaveGrid />
    </div>
  );
}

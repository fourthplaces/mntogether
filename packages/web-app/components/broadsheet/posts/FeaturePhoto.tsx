import type { Post } from '@/lib/broadsheet/types';

interface FeaturePhotoProps {
  data: Post;
}

export function FeaturePhoto({ data: d }: FeaturePhotoProps) {
  const c = 'feat-photo';
  return (
    <div className={c} data-debug={`Post.${c}`}>
      <div
        className={`${c}__image newsprint-photo`}
        style={{ backgroundImage: `url('${d.image || d.media?.image || ''}')` }}
      />
      <div className={`${c}__caption`}>
        <div className={`${c}__text`}>{d.caption || d.media?.caption || ''}</div>
        <div className={`${c}__credit`}>{d.credit || d.media?.credit || ''}</div>
      </div>
    </div>
  );
}

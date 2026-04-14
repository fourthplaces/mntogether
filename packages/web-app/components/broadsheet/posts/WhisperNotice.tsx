import type { Post } from '@/lib/broadsheet/types';
import { MTitle, MBody } from '@/lib/broadsheet/molecules';

interface WhisperNoticeProps {
  data: Post;
}

export function WhisperNotice({ data: d }: WhisperNoticeProps) {
  const c = 'whisper-update';
  return (
    <div className={c} data-debug={`Post.${c}`} data-weight={d.weight}>
      <div className={`${c}__time mono-sm`}>{d.date || ''}</div>
      <MTitle text={d.title} prefix={c} pencilMark={d.pencilMark} />
      <MBody text={d.body} prefix={c} clamp={d.clamp ?? 3} />
    </div>
  );
}

import type { AudioData } from '@/lib/broadsheet/detail-types';
import { Icon } from '@/components/broadsheet/icons/Icon';

export function AudioA({ audio }: { audio: AudioData }) {
  // Generate random waveform bars (deterministic per render)
  const bars = Array.from({ length: 50 }, (_, i) => {
    const h = ((i * 37 + 17) % 80) + 20; // pseudo-random heights
    return <div key={i} className="audio-a__bar" style={{ height: `${h}%` }} />;
  });

  return (
    <div className="audio-a">
      <div className="audio-a__title">{audio.title}</div>
      <div className="audio-a__player">
        <button className="audio-a__play"><Icon name="play-arrow" size={16} /></button>
        <div className="audio-a__waveform">{bars}</div>
        <span className="audio-a__time mono-sm">{audio.currentTime || '0:00'} / {audio.duration}</span>
      </div>
      <div className="audio-a__progress"><div className="audio-a__progress-fill" /></div>
      {audio.credit && <div className="audio-a__credit mono-sm">{audio.credit}</div>}
    </div>
  );
}

export function AudioB({ audio }: { audio: AudioData }) {
  return (
    <div className="audio-b">
      <button className="audio-b__play"><Icon name="play-arrow" size={14} /></button>
      <span className="audio-b__excerpt">{audio.excerpt}</span>
      <a href="#" className="audio-b__listen mono-sm">
        Listen to full interview ({audio.duration}) <Icon name="chevron-right" size={12} />
      </a>
      <div className="audio-b__progress"><div className="audio-b__progress-fill" /></div>
    </div>
  );
}

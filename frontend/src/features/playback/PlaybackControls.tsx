import { SPEEDS, type Playback } from './usePlayback'

interface Props {
  playback: Playback
  label: string | null
}

export function PlaybackControls({ playback, label }: Props) {
  const clock = playback.clock
    ? new Date(playback.clock * 1000).toLocaleTimeString('pt-BR', {
        hour: '2-digit',
        minute: '2-digit',
      })
    : '--:--'

  return (
    <div className="overlay playback">
      <button
        type="button"
        className="playback__button"
        onClick={playback.toggle}
        disabled={!playback.available}
        aria-label={playback.playing ? 'Pausar o dia' : 'Reproduzir o dia'}
      >
        {playback.playing ? '❚❚' : '▶'}
      </button>

      <div>
        <p className="eyebrow">{label ?? 'Escolha um dia'}</p>
        <p className="playback__clock">{clock}</p>
      </div>

      <div className="playback__speeds" role="group" aria-label="Velocidade">
        {SPEEDS.map((speed) => (
          <button
            key={speed}
            type="button"
            className={`chip${playback.speed === speed ? ' chip--on' : ''}`}
            onClick={() => playback.setSpeed(speed)}
          >
            {speed / 60}×
          </button>
        ))}
      </div>
    </div>
  )
}

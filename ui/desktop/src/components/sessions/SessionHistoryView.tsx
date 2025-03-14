import React from 'react';
import { Clock, MessageSquare, Folder } from 'lucide-react';
import { type SessionDetails } from '../../sessions';
import { SessionHeaderCard, SessionMessages } from './SessionViewComponents';

interface SessionHistoryViewProps {
  session: SessionDetails;
  isLoading: boolean;
  error: string | null;
  onBack: () => void;
  onResume: () => void;
  onRetry: () => void;
}

const SessionHistoryView: React.FC<SessionHistoryViewProps> = ({
  session,
  isLoading,
  error,
  onBack,
  onResume,
  onRetry,
}) => {
  return (
    <div className="h-screen w-full">
      <div className="relative flex items-center h-[36px] w-full bg-bgSubtle"></div>

      {/* Top Row - back, info, reopen thread (fixed) */}
      <SessionHeaderCard onBack={onBack}>
        {/* Session info row */}
        <div className="ml-8">
          <h1 className="text-lg font-bold text-textStandard">
            {session.metadata.description || session.session_id}
          </h1>
          <div className="flex items-center text-sm text-textSubtle mt-2 space-x-4">
            <span className="flex items-center">
              <Clock className="w-4 h-4 mr-1" />
              {new Date(session.messages[0]?.created * 1000).toLocaleString()}
            </span>
            <span className="flex items-center">
              <Folder className="w-4 h-4 mr-1" />
              {session.metadata.working_dir}
            </span>
            <span className="flex items-center">
              <MessageSquare className="w-4 h-4 mr-1" />
              {session.metadata.message_count} messages
            </span>
            {session.metadata.total_tokens !== null && (
              <span className="flex items-center">
                {session.metadata.total_tokens.toLocaleString()} tokens
              </span>
            )}
          </div>
        </div>

        <span
          onClick={onResume}
          className="ml-auto text-md cursor-pointer text-textStandard hover:font-bold hover:scale-105 transition-all duration-150"
        >
          Resume Session
        </span>
      </SessionHeaderCard>

      <SessionMessages
        messages={session.messages}
        isLoading={isLoading}
        error={error}
        onRetry={onRetry}
      />
    </div>
  );
};

export default SessionHistoryView;

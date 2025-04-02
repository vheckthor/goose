import React, { useState } from 'react';
import { Modal, ModalContent, ModalHeader, ModalTitle, ModalFooter } from './modal';
import { Label } from './label';
import { Input } from './input';
import { Button } from './button';
import { Clock } from 'lucide-react';

interface ScheduleTaskModalProps {
  onClose: () => void;
  onSchedule: (duration: number, interval: number) => void;
}

export function ScheduleTaskModal({ onClose, onSchedule }: ScheduleTaskModalProps) {
  const [duration, setDuration] = useState<number>(60); // Default 60 minutes
  const [interval, setInterval] = useState<number>(5); // Default 5 minutes

  const handleSchedule = () => {
    onSchedule(duration, interval);
    onClose();
  };

  return (
    <Modal open={true} onOpenChange={onClose}>
      <ModalContent className="sm:max-w-[425px] bg-bgApp border-borderSubtle">
        <ModalHeader>
          <ModalTitle className="flex items-center gap-2 text-textStandard">
            <Clock className="h-5 w-5" />
            Schedule Task
          </ModalTitle>
        </ModalHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="duration" className="text-right text-textStandard">
              Duration (min)
            </Label>
            <Input
              id="duration"
              type="number"
              min={1}
              value={duration}
              onChange={(e) => setDuration(parseInt(e.target.value) || 1)}
              className="col-span-3"
            />
          </div>
          <div className="grid grid-cols-4 items-center gap-4">
            <Label htmlFor="interval" className="text-right text-textStandard">
              Interval (min)
            </Label>
            <Input
              id="interval"
              type="number"
              min={1}
              value={interval}
              onChange={(e) => setInterval(parseInt(e.target.value) || 1)}
              className="col-span-3"
            />
          </div>
          <div className="text-sm text-textSubtle mt-2">
            This will repeat the last command every {interval} minutes for {duration} minutes.
          </div>
        </div>
        <ModalFooter>
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSchedule} className="ml-2">
            Schedule
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
